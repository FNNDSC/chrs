/// An indicator for what mode API items can be accessed.
pub trait Access {}

/// Read-only access
pub struct RoAccess;

/// Read-write access
pub struct RwAccess;

impl Access for RoAccess {}
impl Access for RwAccess {}
