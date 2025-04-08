    mod mock;

    use std::task::Poll;

    use futures::{SinkExt, executor::block_on};
    use mock::ForkAsyncMockSetup;

    #[test]
    fn nothing() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 2>::new();

        let [mut fork1, mut fork2] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
        assert_eq!(fork2.next_a(), Poll::Pending);

    
    }

    #[test]
    fn one_pending_send_one() {
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
    }

    #[test]
    fn both_pending_send_one() {
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
        
         assert_eq!(fork2.next_a(), Poll::Ready(Some(())));
    }
    
    
      #[test]
    fn both_pending_send_two_receive_one() {
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
        block_on(async {
            let _ = sender.send(()).await;
        });
         assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
    }
       #[test]
    fn both_pending_send_two_receive_one_late() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<usize, 2>::new();

        let [mut fork1, mut fork2] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
        assert_eq!(fork2.next_a(), Poll::Pending);
        block_on(async {
            let _ = sender.send(0).await;
        });
        assert_eq!(fork1.next_a(), Poll::Ready(Some(0)));
        block_on(async {
            let _ = sender.send(1).await;
        });
         assert_eq!(fork2.next_a(), Poll::Ready(Some(0)));
    }
    
    
          #[test]
    fn both_pending_send_two_receive_two_twice() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<usize, 2>::new();

        let [mut fork1, mut fork2] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
        assert_eq!(fork2.next_a(), Poll::Pending);
        block_on(async {
            let _ = sender.start_send(0).await;
            let _ = sender.start_send(0).await;
            sender.flush().await;
        });
    
         assert_eq!(fork1.next_a(), Poll::Ready(Some(0)));
          assert_eq!(fork2.next_a(), Poll::Ready(Some(0)));
           assert_eq!(fork1.next_a(), Poll::Ready(Some(1)));
         assert_eq!(fork2.next_a(), Poll::Ready(Some(1)));
    }