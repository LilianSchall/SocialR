use std::collections::HashSet;
use std::ops::Receiver;
use std::sync::{Arc, Barrier, mpsc};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::thread::sleep;
use std::time::Duration;
use crate::worms::Worm;

pub struct Crawler {
    delay: Duration,
    crawling_concurrency: usize,
    processing_concurrency: usize,
}

impl Crawler {
    pub fn new(delay: Duration, crawling_concurrency: usize, processing_concurrency: usize) -> Crawler {
        Crawler {
            delay,
            crawling_concurrency,
            processing_concurrency,
        }
    }


    pub async fn run<T: Send + 'static>(&self, worm: Arc<dyn Worm<Item=T>>) {
        let mut visited_urls = HashSet::<String>::new();
        let craw_concurrency = self.crawling_concurrency;

        // arbitrary values, is subject to refactoring
        let craw_queue_capacity = craw_concurrency * 400;
        let procs_concurrency = self.processing_concurrency;

        // arbitrary values, is subject to refactoring
        let procs_queue_capacity = procs_concurrency * 10;

        let active_worms = Arc::new(AtomicUsize::new(0));

        // these are the tools to establish a communication between the scraper and the worms
        let (urls_tx, urls_rx) = mpsc::channel(craw_queue_capacity);
        let (items_tx, items_rx) = mpsc::channel(procs_queue_capacity);
        let (new_urls_tx, mut new_urls_rx) = mpsc::channel(craw_queue_capacity);

        let barrier = Arc::new(Barrier::new(3));

        for url in worm.start_urls() {
            visited_urls.insert(url.clone());
            let _ = urls_tx.send(url).await;
        }

        self.launch_processors(procs_concurrency, worm.clone(), items_rx, barrier.clone());
        self.launch_scrapers(craw_concurrency, worm.clone(), urls_rx, new_urls_tx.clone(), items_tx,
                             active_worms.clone(), self.delay, barrier.clone());

        // control loop:    queues new URLs that haven't already been visited
        //                  checks if a stop event has been triggered

        self.control_loop(urls_tx.clone(), new_urls_rx, new_urls_tx, &mut visited_urls,
                          active_worms, craw_queue_capacity);

        // dropping the transmitter in order to close the stream
        drop(urls_tx);

        barrier.wait().await;
    }

    fn control_loop(&self, urls_tx: mpsc::Sender<String>,
                    new_urls_rx: mpsc::Receiver<(String, Vec<String>)>, new_urls_tx: mpsc::Sender<(String, Vec<String>)>,
                    visited_urls: &mut HashSet<String>, active_worms: Arc<AtomicUsize>,
                    cq_capacity: usize) {
        loop {
            if let Some((visited_url, new_urls)) = new_urls_rx.try_recv().ok() {
                visited_urls.insert(visited_url);

                for url in new_urls {
                    if !visited_urls.contains(&url) {
                        log::debug!("queueing: {}", url);
                        let _ = urls_tx.send(url);
                    }
                }
            }

            if new_urls_tx.capacity() == cq_capacity &&
                urls_tx.capacity() == cq_capacity &&
                active_worms.load(Ordering::SeqCst) == 0 {

                // the queues are empty and no worm is working at the moment
                // we end the control loop
                break;
            }

            sleep(Duration::from_millis(5));
        }
    }


    fn launch_processors
}