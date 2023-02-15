pub(super) trait IterExt<T, E>: Iterator<Item = Result<T, E>> {
    fn try_find_map<F, B>(&mut self, f: F) -> Option<Result<B, E>>
    where
        F: FnMut(T) -> Option<B>;
}

impl<It, T, E> IterExt<T, E> for It
where
    It: Iterator<Item = Result<T, E>>,
{
    fn try_find_map<F, B>(&mut self, mut f: F) -> Option<Result<B, E>>
    where
        F: FnMut(T) -> Option<B>,
    {
        loop {
            match self.next()?.map(&mut f) {
                Ok(Some(val)) => {
                    break Some(Ok(val));
                }
                Ok(None) => (),
                Err(err) => break Some(Err(err)),
            }
        }
    }
}
