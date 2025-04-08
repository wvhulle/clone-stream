    mod mock;

    use std::task::Poll;

    use futures::{SinkExt, executor::block_on};
    use mock::ForkAsyncMockSetup;





    #[test]
    fn nothing() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 1>::new();

        let [mut fork1] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
    
    }

             
             
                 #[test]
    fn send_one() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 1>::new();

        let [mut fork1] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
        block_on(async {
            let _ = sender.feed(()).await;
            let _ = sender.flush().await;
        });
        assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
      

    
    }

             
                 #[test]
    fn send_one_back_to_pending() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 1>::new();

        let [mut fork1] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
        block_on(async {
            let _ = sender.feed(()).await;
            let _ = sender.flush().await;
        });
        assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
        
        assert_eq!(fork1.next_a(), Poll::Pending);

    
    }

             
                 #[test]
    fn send_two() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 1>::new();

        let [mut fork1] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
        block_on(async {
            let _ = sender.feed(()).await;
            let _ = sender.feed(()).await;
            let _ = sender.flush().await;
        });
        assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
        assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
      

    
    }


              #[test]
    fn send_two_back_to_pending() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 1>::new();

        let [mut fork1] = forks;

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
             