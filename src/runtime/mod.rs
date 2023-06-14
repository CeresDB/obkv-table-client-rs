// Copyright 2022 CeresDB Project Authors. Licensed under Apache-2.0.
// Modifications copyright (C) 2023 OceanBase

#![allow(dead_code)]

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use tokio::{
    runtime::{Builder as RuntimeBuilder, Runtime as TokioRuntime},
    task::JoinHandle as TokioJoinHandle,
};

use crate::{
    error::{Error, Result},
    monitors::runtime_metrics::RuntimeMetrics,
};

pub type RuntimeRef = Arc<Runtime>;

/// A runtime to run future tasks
#[derive(Debug)]
pub struct Runtime {
    rt: TokioRuntime,
    metrics: Arc<RuntimeMetrics>,
}

impl Runtime {
    /// Spawn a future and execute it in this thread pool
    ///
    /// Similar to tokio::runtime::Runtime::spawn()
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        JoinHandle {
            inner: self.rt.spawn(future),
        }
    }

    /// Run the provided function on an executor dedicated to blocking
    /// operations.
    pub fn spawn_blocking<F, R>(&self, func: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        JoinHandle {
            inner: self.rt.spawn_blocking(func),
        }
    }

    /// Run a future to complete, this is the runtime's entry point
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.rt.block_on(future)
    }

    /// Returns the runtime stats
    pub fn stats(&self) -> RuntimeStats {
        RuntimeStats {
            alive_thread_num: self.metrics.get_runtime_thread_alive_gauges(),
            idle_thread_num: self.metrics.get_runtime_thread_idle_gauges(),
        }
    }
}

pin_project! {
    #[derive(Debug)]
    pub struct JoinHandle<T> {
        #[pin]
        inner: TokioJoinHandle<T>,
    }
}

impl<T> JoinHandle<T> {
    pub fn abort(&self) {
        self.inner.abort();
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.inner.poll(ctx).map_err(Error::JoinTask)
    }
}

/// Helper that aborts the given join handles on drop.
///
/// Useful to kill background tasks when the consumer is dropped.
#[derive(Debug)]
pub struct AbortOnDropMany<T>(pub Vec<JoinHandle<T>>);

impl<T> Drop for AbortOnDropMany<T> {
    fn drop(&mut self) {
        for join_handle in &self.0 {
            join_handle.inner.abort();
        }
    }
}

/// Runtime statistics
pub struct RuntimeStats {
    pub alive_thread_num: i64,
    pub idle_thread_num: i64,
}

pub struct Builder {
    thread_name: String,
    builder: RuntimeBuilder,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            thread_name: "runtime-default".to_string(),
            builder: RuntimeBuilder::new_multi_thread(),
        }
    }
}

fn with_metrics<F>(metrics: &Arc<RuntimeMetrics>, f: F) -> impl Fn()
where
    F: Fn(&Arc<RuntimeMetrics>) + 'static,
{
    let m = metrics.clone();
    move || {
        f(&m);
    }
}

impl Builder {
    /// Sets the number of worker threads the Runtime will use.
    ///
    /// This can be any number above 0
    pub fn worker_threads(&mut self, val: usize) -> &mut Self {
        self.builder.worker_threads(val);
        self
    }

    /// Sets name of threads spawned by the Runtime thread pool
    pub fn thread_name(&mut self, val: impl Into<String>) -> &mut Self {
        self.thread_name = val.into();
        self
    }

    /// Enable all feature of the underlying runtime
    pub fn enable_all(&mut self) -> &mut Self {
        self.builder.enable_all();
        self
    }

    pub fn build(&mut self) -> Result<Runtime> {
        let metrics = Arc::new(RuntimeMetrics::new(&self.thread_name));

        let rt = self
            .builder
            .thread_name(self.thread_name.clone())
            .on_thread_start(with_metrics(&metrics, |m| {
                m.on_thread_start();
            }))
            .on_thread_stop(with_metrics(&metrics, |m| {
                m.on_thread_stop();
            }))
            .on_thread_park(with_metrics(&metrics, |m| {
                m.on_thread_park();
            }))
            .on_thread_unpark(with_metrics(&metrics, |m| {
                m.on_thread_unpark();
            }))
            .build()?;

        Ok(Runtime { rt, metrics })
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread, time::Duration};

    use tokio::sync::oneshot;
    use tokio_test::assert_ok;

    use super::*;

    fn rt() -> Arc<Runtime> {
        let rt = Builder::default()
            .worker_threads(2)
            .thread_name("test_spawn_join")
            .enable_all()
            .build();
        assert!(rt.is_ok());
        Arc::new(rt.unwrap())
    }

    #[test]
    fn test_stats() {
        let rt = Builder::default()
            .worker_threads(5)
            .thread_name("test_stats")
            .enable_all()
            .build();
        assert!(rt.is_ok());
        let rt: RuntimeRef = Arc::new(rt.unwrap());
        // wait threads created
        thread::sleep(Duration::from_millis(50));

        let s = rt.stats();
        assert_eq!(5, s.alive_thread_num);
        assert_eq!(5, s.idle_thread_num);

        rt.spawn(async {
            thread::sleep(Duration::from_millis(50));
        });

        thread::sleep(Duration::from_millis(10));
        let s = rt.stats();
        assert_eq!(5, s.alive_thread_num);
        assert_eq!(4, s.idle_thread_num);
    }

    #[test]
    fn block_on_async() {
        let rt = rt();

        let out = rt.block_on(async {
            let (tx, rx) = oneshot::channel();

            thread::spawn(move || {
                thread::sleep(Duration::from_millis(50));
                tx.send("ZOMG").unwrap();
            });

            assert_ok!(rx.await)
        });

        assert_eq!(out, "ZOMG");
    }

    #[test]
    fn spawn_from_blocking() {
        let rt = rt();
        let rt1 = rt.clone();
        let out = rt.block_on(async move {
            let rt2 = rt1.clone();
            let inner = assert_ok!(
                rt1.spawn_blocking(move || { rt2.spawn(async move { "hello" }) })
                    .await
            );

            assert_ok!(inner.await)
        });

        assert_eq!(out, "hello")
    }

    #[test]
    fn test_spawn_join() {
        let rt = rt();
        let handle = rt.spawn(async { 1 + 1 });

        assert_eq!(2, rt.block_on(handle).unwrap());
    }
}
