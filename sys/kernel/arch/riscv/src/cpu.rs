pub enum PrivilegeLevel {
    Supervisor,
    User,
}

#[repr(C)]
pub enum ExtensionState {
    Off = 0,
    Initial,
    Clean,
    Dirty,
}

boltk_macros::bitstruct! {
    pub struct Sstatus : usize {
        /// Global Interrupt Enable
        pub const SIE       = 1, 1;
        /// Previous Interrupt Enable
        pub const SPIE      = 1, 5;
        /// U-mode Big Endian
        pub const UBE       = 1, 6;
        /// Previous Privilege Level
        pub const SPP       = 1, 8;
        /// Vector Extension State
        pub const VS        = 2, 9;
        /// Floating-Point State
        pub const FS        = 2, 13;
        /// User-Mode Extensions State
        pub const XS        = 2, 15;
        /// Supervisor User Memory
        ///
        ///
        pub const SUM       = 1, 18;
        /// Make Executable Readable
        ///
        /// When set, allows reads from pages only marked executable.
        pub const MXR       = 1, 19;
        /// User-Mode XLEN
        ///
        ///
        pub const UXL       = 2, 32;
        /// State Dirty
        ///
        /// This bit provides an indication of the extension state. It is set if any
        /// of [`FS`], [`VS`], or [`XS`] indicates dirty state.
        pub const SD        = 1, 63;
    }
}

impl Sstatus {
    /// Returns the privilege level of the hart before a trap into S-mode
    pub const fn previous_privilege_level(self) -> PrivilegeLevel {
        if self.contains(Self::SPP) {
            PrivilegeLevel::Supervisor
        } else {
            PrivilegeLevel::User
        }
    }

    /// Returns `true` if any extensions have dirty state which needs handling
    ///
    /// This provides a hint to whether [`vector_state()`], [`fpu_state()`], or
    /// [`extension_state()`] would indicate some dirty state.
    ///
    /// Note that this mechanism is provided by the hardware, so this function is extremely
    /// inexpensive.
    pub const fn dirty_state(self) -> bool {
        self.bits & (1 << (usize::BITS - 1)) != 0
    }

    /// Returns a summary of the Vector extension state
    ///
    /// See [`ExtensionState`] for more information.
    pub const fn vector_state(self) -> ExtensionState {
        match self.bits >> 9 & 0x3 {
            0 => ExtensionState::Off,
            1 => ExtensionState::Initial,
            2 => ExtensionState::Clean,
            3 => ExtensionState::Dirty,
            _ => unreachable!(),
        }
    }

    /// Returns a summary of the floating-point state
    ///
    /// See [`ExtensionState`] for more information.
    pub const fn fpu_state(self) -> ExtensionState {
        match self.bits >> 13 & 0x3 {
            0 => ExtensionState::Off,
            1 => ExtensionState::Initial,
            2 => ExtensionState::Clean,
            3 => ExtensionState::Dirty,
            _ => unreachable!(),
        }
    }

    /// Returns a summary of the state of various extensions
    ///
    /// See [`ExtensionState`] for more information.
    pub const fn extension_state(self) -> ExtensionState {
        match self.bits >> 15 & 0x3 {
            0 => ExtensionState::Off,
            1 => ExtensionState::Initial,
            2 => ExtensionState::Clean,
            3 => ExtensionState::Dirty,
            _ => unreachable!(),
        }
    }
}
