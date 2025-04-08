    mod mock;

    use std::task::Poll;

    use futures::{SinkExt, executor::block_on};
    use mock::ForkAsyncMockSetup;

    #[test]
    fn s1p_s2p_s_s1r_s1p_s2r_s2p() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 2>::new();

        let [mut fork1, mut fork2] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
        assert_eq!(fork2.next_a(), Poll::Pending);

        block_on(async {
            let _ = sender.send(()).await;
        });
        assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
        assert_eq!(fork1.next_a(), Poll::Pending);

        assert_eq!(fork2.next_a(), Poll::Ready(Some(())));

        assert_eq!(fork2.next_a(), Poll::Pending);
    }

    #[test]
    fn second_pending() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 2>::new();

        let [mut fork1, mut fork2] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
        block_on(async {
            let _ = sender.send(()).await;
        });
        assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
        assert_eq!(fork1.next_a(), Poll::Pending);

        assert_eq!(fork2.next_a(), Poll::Pending);
    }

    #[test]
    fn second_later_ready() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 2>::new();

        let [mut fork1, mut fork2] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
        block_on(async {
            let _ = sender.send(()).await;
        });
        assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
        assert_eq!(fork2.next_a(), Poll::Pending);

        block_on(async {
            let _ = sender.send(()).await;
        });

        assert_eq!(fork2.next_a(), Poll::Ready(Some(())));

        assert_eq!(fork1.next_a(), Poll::Pending);

        assert_eq!(fork2.next_a(), Poll::Pending);
    }

    #[test]
    fn multi() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 2>::new();

        let [mut fork1, _] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
        block_on(async {
            let _ = sender.feed(()).await;
            let _ = sender.feed(()).await;
            let _ = sender.flush().await;
        });
        assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
        assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
        assert_eq!(fork1.next_a(), Poll::Pending);

    
    }

    #[test]
    fn multi_both() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 2>::new();

        assert_eq!(fork2.next_a(), Poll::Pending);

        block_on(async {
            let _ = sender.feed(()).await;
            let _ = sender.feed(()).await;
            let _ = sender.flush().await;
        });

        assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
        assert_eq!(fork2.next_a(), Poll::Ready(Some(())));
assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
        assert_eq!(fork2.next_a(), Poll::Ready(Some(())));
        assert_eq!(fork1.next_a(), Poll::Pending);

        assert_eq!(fork2.next_a(), Poll::Pending);
    }

    #[test]
    fn multi_both_interleave() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 2>::new();

        let [mut fork1, mut fork2] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
        assert_eq!(fork1.next_b(), Poll::Pending);
        assert_eq!(fork2.next_a(), Poll::Pending);

        block_on(async {
            let _ = sender.feed(()).await;
            let _ = sender.feed(()).await;
            let _ = sender.flush().await;
        });

        assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
        assert_eq!(fork2.next_a(), Poll::Ready(Some(())));

       assert_eq!(fork1.next_b(), Poll::Ready(Some(())));

        assert_eq!(fork1.next_a(), Poll::Pending);
   assert_eq!(fork1.next_b(), Poll::Pending);

        assert_eq!(fork2.next_a(), Poll::Pending);
    }
