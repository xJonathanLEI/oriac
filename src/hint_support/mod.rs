use crate::cairo::lang::vm::{
    memory_segments::MemorySegmentManager, relocatable::RelocatableValue,
    validated_memory_dict::ValidatedMemoryDict,
};

use rustpython_vm::{
    builtins::PyTypeRef, pyclass, pyimpl, Context, PyPayload, PyRef, VirtualMachine as PythonVm,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct StaticLocals {
    pub segments: Rc<RefCell<MemorySegmentManager>>,
}

#[pyclass(name = "RelocatableValue", module = false)]
#[derive(Debug, PyPayload)]
pub struct PyRelocatableValue {
    pub inner: RelocatableValue,
}

#[pyclass(name = "MemorySegmentManager", module = false)]
#[derive(Debug, PyPayload)]
pub struct PyMemorySegmentManager {
    pub inner: Rc<RefCell<MemorySegmentManager>>,
}

#[pyclass(name = "ValidatedMemoryDict", module = false)]
#[derive(Debug, PyPayload)]
pub struct PyValidatedMemoryDict {
    pub inner: Rc<RefCell<ValidatedMemoryDict>>,
}

#[pyimpl]
impl PyRelocatableValue {
    pub fn from_relocatable_value(value: &RelocatableValue) -> Self {
        Self {
            inner: value.to_owned(),
        }
    }

    pub fn to_relocatable_value(&self) -> RelocatableValue {
        self.inner.to_owned()
    }
}

#[pyimpl]
impl PyMemorySegmentManager {
    pub fn py_add(zelf: PyRef<Self>, vm: &PythonVm) -> PyRef<PyRelocatableValue> {
        PyRelocatableValue::from_relocatable_value(&zelf.inner.borrow_mut().add(None)).into_ref(vm)
    }

    #[extend_class]
    fn extend_class_with_fields(ctx: &Context, class: &PyTypeRef) {
        class.set_str_attr("add", ctx.new_method("add", class.to_owned(), Self::py_add));
    }
}

#[pyimpl]
impl PyValidatedMemoryDict {
    pub fn py_setitem(
        zelf: PyRef<Self>,
        addr: PyRef<PyRelocatableValue>,
        value: PyRef<PyRelocatableValue>,
    ) {
        zelf.inner.borrow_mut().index_set(
            addr.to_relocatable_value().into(),
            value.to_relocatable_value().into(),
        );
    }
}
