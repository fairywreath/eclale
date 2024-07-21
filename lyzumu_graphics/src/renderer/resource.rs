use anyhow::Result;

pub(crate) struct MultiBufferedResource<T, const N: usize> {
    resources: [T; N],
}

impl<T, const N: usize> MultiBufferedResource<T, N> {
    pub(crate) fn new<F>(mut init_fn: F) -> Result<Self>
    where
        F: FnMut() -> T,
    {
        let resources: [T; N] = std::array::from_fn(|_| init_fn());
        Ok(MultiBufferedResource { resources })
    }

    pub(crate) fn get(&self, index: usize) -> &T {
        &self.resources[index % N]
    }

    pub(crate) fn get_mut(&mut self, index: usize) -> &mut T {
        &mut self.resources[index % N]
    }
}
