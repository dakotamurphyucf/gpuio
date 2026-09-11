use binprot::macros::BinProtWrite;

macro_rules! identifier {
    ($name:ident) => {
        /// A slot plus generation. Equal slots alone never establish identity.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, BinProtWrite)]
        pub struct $name {
            slot: i64,
            generation: i64,
        }

        impl $name {
            pub fn from_parts(slot: i64, generation: i64) -> Option<Self> {
                if (0..=u32::MAX as i64).contains(&slot)
                    && (1..=u32::MAX as i64).contains(&generation)
                {
                    Some(Self { slot, generation })
                } else {
                    None
                }
            }

            pub fn slot(self) -> usize {
                self.slot as usize
            }

            pub fn generation(self) -> u32 {
                self.generation as u32
            }
        }
    };
}

identifier!(WindowId);
identifier!(NodeId);
identifier!(HandlerId);
identifier!(ResourceId);
