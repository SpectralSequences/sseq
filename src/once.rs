use spin;

pub struct Once<T> {
    once : spin::Once<T>
} 

impl<T> Once<T> {
    pub fn new() -> Self {
        Self {
            once : spin::Once::new()
        }
    }  

    pub fn get(&self) -> &T {
        self.once.r#try().expect("Value hasn't been set yet.")
    }

    pub fn set(&self, value : T){
        let mut ran = false;
        let _result = self.once.call_once(|| {
            ran = true;
            value
        });
        assert!(ran, "Value was already set.");
    }

    pub fn call_once(&self, f : Box<FnOnce() -> T>){
        self.once.call_once(f);
    }

    pub fn get_option(&self) -> Option<&T> {
        self.once.r#try()
    }

    pub fn has(&self) -> bool {
        self.once.r#try().is_some()
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    
    #[test]
    #[should_panic(expected = "Value hasn't been set yet.")]
    fn test_once_failed_get(){
        let once : OnceRefOwned<u32> = OnceRefOwned::new();
        once.get();
    }

    #[test]
    #[should_panic(expected = "Value was already set.")]
    fn test_once_failed_set(){
        let once : OnceRefOwned<u32> = OnceRefOwned::new();
        once.set(5);
        once.set(5);
    }

    #[test]
    fn test_once_set_get(){
        let once : OnceRefOwned<u32> = OnceRefOwned::new();
        once.set(5);
        assert!(*once.get() == 5);
    }    
}
