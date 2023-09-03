#![forbid(unsafe_code)]

use crate::object::Store;
use crate::{
    data::ObjectId,
    error::{Error, NotFoundError, Result},
    object::{Object, Schema},
    storage::StorageTransaction,
};
use std::ops::Deref;
use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    marker::PhantomData,
    rc::Rc,
};

////////////////////////////////////////////////////////////////////////////////

pub struct Transaction<'a> {
    inner: Box<dyn StorageTransaction + 'a>,
    cache: RefCell<HashMap<ObjectId, Rc<RefCell<dyn Store>>>>,
    states: RefCell<HashMap<ObjectId, Rc<RefCell<ObjectState>>>>,
}

impl<'a> Transaction<'a> {
    pub(crate) fn new(inner: Box<dyn StorageTransaction + 'a>) -> Self {
        Self {
            inner,
            cache: RefCell::new(HashMap::new()),
            states: RefCell::new(HashMap::new()),
        }
    }

    fn ensure_table(&self, schema: &Schema) -> Result<()> {
        if !self.inner.table_exists(schema.get_table_name())? {
            self.inner.create_table(schema)?;
        }
        Ok(())
    }

    pub fn create<T: Object>(&self, src_obj: T) -> Result<Tx<'_, T>> {
        // Insert object into the underlying database.
        let schema = <T as Object>::describe();
        self.ensure_table(&schema)?;
        let id = self
            .inner
            .insert_row(&schema, src_obj.as_row().as_slice())?;

        // Create Tx object and save it in the transaction cache.
        let rc = Rc::new(RefCell::new(src_obj));
        let state = Rc::new(RefCell::new(ObjectState::Clean));
        self.cache
            .borrow_mut()
            .insert(id, rc.clone() as Rc<RefCell<dyn Store>>);
        self.states.borrow_mut().insert(id, state.clone());
        Ok(Tx::new(rc, id, state))
    }

    pub fn get<T: Object>(&self, id: ObjectId) -> Result<Tx<'_, T>> {
        // If current transaction already has such object loaded than return it.
        let (rc, state_ref) = if self.cache.borrow().contains_key(&id) {
            // Check if an object was removed already.
            if *self.states.borrow().get(&id).unwrap().deref().borrow() == ObjectState::Removed {
                return Err(Error::NotFound(Box::new(NotFoundError {
                    object_id: id,
                    type_name: <T as Object>::type_name(),
                })));
            }
            let rc = self.cache.borrow().get(&id).unwrap().clone();
            let state = self.states.borrow().get(&id).unwrap().clone();
            (rc, state)
        } else {
            // Get object from underlying database.
            let schema = <T as Object>::describe();
            self.ensure_table(&schema)?;
            let row = self.inner.select_row(id, &schema)?;
            let src_obj = <T as Object>::from_row(row);

            // Create Tx object and save it in the transaction cache.
            let rc = Rc::new(RefCell::new(src_obj)) as Rc<RefCell<dyn Store>>;
            let state = Rc::new(RefCell::new(ObjectState::Clean));
            self.cache.borrow_mut().insert(id, rc.clone());
            self.states.borrow_mut().insert(id, state.clone());
            (rc, state)
        };
        Ok(Tx::new(rc, id, state_ref))
    }

    pub fn commit(self) -> Result<()> {
        for (id, state) in self.states.borrow().iter() {
            let cache = self.cache.borrow();
            match *state.deref().borrow() {
                ObjectState::Modified => {
                    let object = cache.get(id).unwrap().deref().borrow();
                    let row = object.as_row();
                    let schema = object.describe();
                    self.inner.update_row(*id, &schema, row.as_slice())?;
                }
                ObjectState::Removed => {
                    let object = cache.get(id).unwrap().deref().borrow();
                    let schema = object.describe();
                    self.inner.delete_row(*id, &schema)?;
                }
                ObjectState::Clean => (),
            }
        }
        self.inner.commit()
    }

    pub fn rollback(self) -> Result<()> {
        self.inner.rollback()
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ObjectState {
    Clean,
    Modified,
    Removed,
}

#[derive(Clone)]
pub struct Tx<'a, T> {
    state: Rc<RefCell<ObjectState>>,
    object: Rc<RefCell<dyn Store>>,
    id: ObjectId,
    lifetime: PhantomData<&'a T>,
}

impl<'a, T: Any> Tx<'a, T> {
    fn new(object: Rc<RefCell<dyn Store>>, id: ObjectId, state: Rc<RefCell<ObjectState>>) -> Self {
        Self {
            state,
            object,
            id,
            lifetime: PhantomData,
        }
    }
    pub fn id(&self) -> ObjectId {
        self.id
    }

    pub fn state(&self) -> ObjectState {
        *self.state.deref().borrow()
    }

    pub fn borrow(&self) -> Ref<'_, T> {
        if *self.state.deref().borrow() == ObjectState::Removed {
            panic!("cannot borrow a removed object")
        }
        let borrowed = self.object.deref().borrow();
        Ref::map(borrowed, |x| x.as_any().downcast_ref::<T>().unwrap())
    }

    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        // self.object.deref().borrow_mut()
        if *self.state.deref().borrow() == ObjectState::Removed {
            panic!("cannot borrow a removed object")
        }
        *self.state.borrow_mut() = ObjectState::Modified;
        let borrowed = self.object.deref().borrow_mut();
        RefMut::map(borrowed, |x| x.as_mut_any().downcast_mut::<T>().unwrap())
    }

    pub fn delete(self) {
        if self.object.try_borrow_mut().is_err() {
            panic!("cannot delete a borrowed object")
        }
        *self.state.borrow_mut() = ObjectState::Removed;
    }
}
