pub type Observer = Box<dyn Fn()>;

#[derive(Default)]
pub struct ObserverList(Vec<Option<Observer>>);
impl std::fmt::Debug for ObserverList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ObserverList").field(&self.0.len()).finish()
    }
}

impl ObserverList {
    pub fn add_observer(&mut self, observer: Observer) -> usize {
        let vacant = self.0.iter()
                           .enumerate()
                           .find_map(|(i,e)| {
                               match e {
                                   Some(_) => None,
                                   None => Some(i),
                               }
                           });
        match vacant {
            Some(i) => { self.0[i] = Some(observer); i },
            None => { self.0.push(Some(observer)); self.0.len() - 1 }
        }
    }

    pub fn remove_observer(&mut self, observer_key: usize) {
        self.0.get_mut(observer_key).map(|e| *e = None);
    }

    pub fn invalidate_for_observers(&self) {
        self.0.iter().for_each(|e| {
            e.as_ref().map(|o| o());
        });
    }
}

pub trait HasObserverList {
    fn get_observer_list(&self) -> &ObserverList;
    fn get_mut_observer_list(&mut self) -> &mut ObserverList;
}

impl<T> Observable for T where T: HasObserverList {
    fn add_observer(&mut self, observer: Observer) -> usize {
        self.get_mut_observer_list().add_observer(observer)
    }

    fn remove_observer(&mut self, observer_key: usize) {
        self.get_mut_observer_list().remove_observer(observer_key);
    }

    fn invalidate_for_observers(&self) {
        self.get_observer_list().invalidate_for_observers();
    }
}

pub trait Observable {
    fn add_observer(&mut self, observer: Observer) -> usize;
    fn remove_observer(&mut self, observer_key: usize);
    fn invalidate_for_observers(&self);
}
