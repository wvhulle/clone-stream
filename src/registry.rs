use std::task::Waker;

use log::{trace, warn};

use crate::{
    error::{CloneStreamError, Result},
    states::CloneState,
};

#[derive(Debug)]
pub(crate) struct CloneRegistry {
    clones: Vec<Option<CloneState>>,
    available_indices: Vec<usize>,
    max_clone_count: usize,
}

impl CloneRegistry {
    pub(crate) fn new(max_clone_count: usize) -> Self {
        Self {
            clones: Vec::new(),
            available_indices: Vec::new(),
            max_clone_count,
        }
    }

    pub(crate) fn register(&mut self) -> Result<usize> {
        if self.count() >= self.max_clone_count {
            return Err(CloneStreamError::MaxClonesExceeded {
                current_count: self.count(),
                max_allowed: self.max_clone_count,
            });
        }

        if let Some(reused_id) = self.available_indices.pop() {
            trace!("Registering clone {reused_id} (reused index).");
            self.clones[reused_id] = Some(CloneState::default());
            Ok(reused_id)
        } else {
            let clone_id = self.clones.len();
            trace!("Registering clone {clone_id} (new index).");
            self.clones.push(Some(CloneState::default()));
            Ok(clone_id)
        }
    }

    pub(crate) fn unregister(&mut self, clone_id: usize) {
        trace!("Unregistering clone {clone_id}.");

        if !self.exists(clone_id) {
            warn!("Attempted to unregister clone {clone_id} that was not registered");
            return;
        }

        self.clones[clone_id] = None;
        self.available_indices.push(clone_id);
        trace!("Unregister of clone {clone_id} complete.");
    }

    pub(crate) fn take(&mut self, clone_id: usize) -> Option<CloneState> {
        self.clones.get_mut(clone_id)?.take()
    }

    pub(crate) fn restore(&mut self, clone_id: usize, state: CloneState) -> Result<()> {
        if clone_id >= self.clones.len() {
            warn!("Attempted to restore clone {clone_id} with invalid ID (out of bounds)");
            return Err(CloneStreamError::InvalidCloneId { clone_id });
        }

        if self.clones[clone_id].is_some() {
            warn!("Attempted to restore clone {clone_id} that is already active");
            return Err(CloneStreamError::CloneAlreadyActive { clone_id });
        }

        self.clones[clone_id] = Some(state);
        trace!("Restored clone {clone_id}");
        Ok(())
    }

    pub(crate) fn exists(&self, clone_id: usize) -> bool {
        clone_id < self.clones.len() && self.clones[clone_id].is_some()
    }

    pub(crate) fn count(&self) -> usize {
        self.clones.iter().filter(|s| s.is_some()).count()
    }

    pub(crate) fn iter_active_with_ids(&self) -> impl Iterator<Item = (usize, &CloneState)> {
        self.clones
            .iter()
            .enumerate()
            .filter_map(|(id, state_opt)| state_opt.as_ref().map(|state| (id, state)))
    }

    pub(crate) fn iter_active(&self) -> impl Iterator<Item = &CloneState> {
        self.clones
            .iter()
            .filter_map(|state_opt| state_opt.as_ref())
    }

    pub(crate) fn collect_wakers_needing_base_item(&self) -> Vec<Waker> {
        trace!("Collecting wakers for clones needing base item.");
        self.iter_active()
            .filter(|state| state.should_still_see_base_item())
            .filter_map(CloneState::waker)
            .collect()
    }

    pub(crate) fn has_other_clones_waiting(&self, exclude_clone_id: usize) -> bool {
        self.clones.iter().enumerate().any(|(clone_id, state_opt)| {
            clone_id != exclude_clone_id
                && state_opt
                    .as_ref()
                    .is_some_and(CloneState::should_still_see_base_item)
        })
    }

    pub(crate) fn get_clone_state(&self, clone_id: usize) -> Option<&CloneState> {
        self.clones.get(clone_id).and_then(|opt| opt.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_respects_max_clone_limit_with_index_reuse() {
        let mut registry = CloneRegistry::new(1);

        trace!("Register and immediately unregister to create available_indices");
        let id1 = registry.register().unwrap();
        registry.unregister(id1);

        trace!("At this point: count = 0, available_indices = [0]");

        trace!("Register again to get to max capacity");
        let _id2 = registry.register().unwrap();
        assert_eq!(
            registry.count(),
            1,
            "Registry should have exactly 1 active clone after registering with reused index"
        );

        match registry.register() {
            Ok(_) => panic!("Should have failed - already at max capacity!"),
            Err(CloneStreamError::MaxClonesExceeded {
                current_count,
                max_allowed,
            }) => {
                assert_eq!(current_count, 1, "Error should report current count of 1");
                assert_eq!(max_allowed, 1, "Error should report max allowed of 1");
            }
            Err(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    #[test]
    fn test_index_reuse_works_when_under_limit() {
        let mut registry = CloneRegistry::new(2);
        let a = registry.register().unwrap();
        let _b = registry.register().unwrap();
        trace!("Creates available index");
        registry.unregister(a);

        trace!("Now at count=1, available_indices=[0], max=2");
        trace!("Should reuse index 0");
        let _c = registry.register().unwrap();
        assert_eq!(
            registry.count(),
            2,
            "Registry should have exactly 2 active clones after index reuse"
        );

        match registry.register() {
            Ok(_) => panic!("Should have failed - we're at max capacity!"),
            Err(CloneStreamError::MaxClonesExceeded {
                current_count,
                max_allowed,
            }) => {
                assert_eq!(current_count, 2, "Error should report current count of 2");
                assert_eq!(max_allowed, 2, "Error should report max allowed of 2");
            }
            Err(e) => panic!("Unexpected error: {e:?}"),
        }
    }
}
