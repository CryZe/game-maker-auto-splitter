/// CInstance
pub mod c_instance {
    /// CObjectGM* m_pObject
    pub const P_OBJECT: u64 = 0x90;
    /// int i_id
    pub const ID: u64 = 0xb4;
}

/// CObjectGM
pub mod c_object_gm {
    /// char* m_pName
    pub const P_NAME: u64 = 0x00;
}

/// YYObjectBase
pub mod yy_object_base {
    /// CHashMap<int,_RValue_*,_3>* m_yyvarsMap
    pub const YY_VARS_MAP: u64 = 0x48;
}

/// RValue
pub mod r_value {
    // /// union field_0
    // pub const FIELD_0: u64 = 0x0;
    // /// uint flags
    // pub const FLAGS: u64 = 0x8;
    /// uint kind
    pub const KIND: u64 = 0xc;
}

/// CRoom
pub mod c_room {
    /// OLinkedList<CInstance> m_Active
    pub const M_ACTIVE: u64 = 0xd8;
}

/// OLinkedList
pub mod o_linked_list {
    /// T* m_PFirst
    pub const M_P_FIRST: u64 = 0x0;
}
