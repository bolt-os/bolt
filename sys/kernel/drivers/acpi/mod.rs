/*
 * Copyright (c) 2023 xvanc <xvancm@gmail.com>
 *
 * Redistribution and use in source and binary forms, with or without modification,
 * are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice,
 *    this list of conditions and the following disclaimer.
 *
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 * 3. Neither the name of the copyright holder nor the names of its contributors
 *    may be used to endorse or promote products derived from this software without
 *    specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY
 * EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * SPDX-License-Identifier: BSD-3-Clause
 */

mod lai;

use alloc::sync::Arc;

use ::acpi::{sdt::mcfg::Mcfg, RootTable};
use lai::{NodeKind, Value};

use crate::{
    dev::{
        self,
        resource::{IoResource, IrqFlags, IrqResource},
        Device,
    },
    drivers::pcie,
    intr::{Polarity, Trigger},
    sync::{lazy::Lazy, RwLock},
    vm::{PhysAddr, VirtAddr},
};

#[derive(Clone, Copy)]
struct AcpiBridge;

impl acpi::Bridge for AcpiBridge {
    fn map(&self, phys: usize, _size: usize) -> usize {
        PhysAddr::new(phys).to_virt().into()
    }

    fn remap(&self, virt: usize, _new_size: usize) -> usize {
        virt
    }

    fn unmap(&self, _virt: usize) {}
}

static ACPI_ROOT: Lazy<RwLock<RootTable<AcpiBridge>>> = Lazy::new(|| unimplemented!());

#[derive(Debug)]
pub enum ResourceKind {
    Memory,
    Io,
    BusNumberRange,
    Reserved(u8),
    Vendor(u8),
}

impl ResourceKind {
    pub const fn from_raw(kind: u8) -> Self {
        match kind {
            0 => Self::Memory,
            1 => Self::Io,
            2 => Self::BusNumberRange,
            3..=191 => Self::Reserved(kind),
            192..=255 => Self::Vendor(kind),
        }
    }
}

struct Parser<'a> {
    data: &'a [u8],
    pos:  usize,
}

impl<'a> Parser<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn bump(&mut self) -> ParseResult<u8> {
        if self.pos < self.data.len() {
            let byte = self.data[self.pos];
            self.pos += 1;
            Ok(byte)
        } else {
            Err(ParseError::OutOfData)
        }
    }

    fn parse_slice(&mut self, len: usize) -> ParseResult<&'a [u8]> {
        let data = self
            .data
            .get(self.pos..self.pos + len)
            .ok_or(ParseError::OutOfData)?;
        self.pos += len;
        Ok(data)
    }

    fn parse_le_u16(&mut self) -> ParseResult<u16> {
        let data = self.parse_slice(2)?;
        Ok(unsafe { data.as_ptr().cast::<u16>().read_unaligned() })
    }

    fn parse_le_u32(&mut self) -> ParseResult<u32> {
        let data = self.parse_slice(4)?;
        Ok(unsafe { data.as_ptr().cast::<u32>().read_unaligned() })
    }

    fn parse_le_u64(&mut self) -> ParseResult<u64> {
        let data = self.parse_slice(4)?;
        Ok(unsafe { data.as_ptr().cast::<u64>().read_unaligned() })
    }
}

#[derive(Debug)]
enum ParseError {
    OutOfData,
}

type ParseResult<T> = core::result::Result<T, ParseError>;

#[allow(clippy::too_many_lines)]
fn parse_inner(p: &mut Parser, resources: &mut Vec<dev::Resource>) -> ParseResult<bool> {
    let first = p.bump()?;

    if first & 1 << 7 == 0 {
        // small

        let tag = first >> 3 & 0xf;
        let len = first & 0x7;

        match tag {
            // IRQ
            0x4 => {
                let mask = p.parse_le_u16()?;
                for irq in (0..16).filter(|bit| mask & 1 << bit != 0) {
                    let mut trigger = Trigger::Edge;
                    let mut polarity = Polarity::High;
                    let mut flags = IrqFlags::empty();

                    if len == 3 {
                        let info = p.bump()?;

                        if info & 1 << 0 == 0 {
                            trigger = Trigger::Level;
                        }
                        if info & 1 << 3 != 0 {
                            polarity = Polarity::Low;
                        }
                        flags.set(IrqFlags::SHARED, info & 1 << 4 != 0);
                        flags.set(IrqFlags::WAKE_CAPABLE, info & 1 << 5 != 0);
                    }

                    resources.push(dev::Resource::Irq(IrqResource {
                        irq,
                        flags,
                        trigger,
                        polarity,
                    }));
                }
            }
            // I/O Port
            0x8 => {
                let _info = p.bump()?;
                let min = p.parse_le_u16()?;
                let _max = p.parse_le_u16()?;
                let _align = p.bump()?;
                let len = p.bump()?;

                resources.push(dev::Resource::Io(IoResource {
                    base: min as _,
                    size: len as _,
                }));
            }
            // Fixed Location I/O Port
            0x9 => {
                let base = p.parse_le_u16()?;
                let len = p.bump()?;

                resources.push(dev::Resource::Io(IoResource {
                    base: base as _,
                    size: len as _,
                }));
            }
            // End Tag
            0xf => {
                p.bump()?;
                return Ok(false);
            }

            _ => {}
        }
    } else {
        // large

        let tag = first & 0x7f;
        let _len = p.parse_le_u16()?;

        match tag {
            // 32-bit Fixed Memory Range
            0x6 => {
                let _info = p.bump()?;
                resources.push(dev::Resource::memory(
                    p.parse_le_u32()? as usize,
                    p.parse_le_u32()? as usize,
                ));
            }
            // Address Space
            0x7 | 0x8 | 0xa => {
                let kind = ResourceKind::from_raw(p.bump()?);
                let _flags = p.bump()?;
                let _kind_flags = p.bump()?;
                let (_granularity, base, _max, _offset, size) = match tag {
                    0x7 => (
                        p.parse_le_u32()? as usize,
                        p.parse_le_u32()? as usize,
                        p.parse_le_u32()? as usize,
                        p.parse_le_u32()? as usize,
                        p.parse_le_u32()? as usize,
                    ),
                    0x8 => (
                        p.parse_le_u16()? as usize,
                        p.parse_le_u16()? as usize,
                        p.parse_le_u16()? as usize,
                        p.parse_le_u16()? as usize,
                        p.parse_le_u16()? as usize,
                    ),
                    0xa => (
                        p.parse_le_u64()? as usize,
                        p.parse_le_u64()? as usize,
                        p.parse_le_u64()? as usize,
                        p.parse_le_u64()? as usize,
                        p.parse_le_u64()? as usize,
                    ),
                    _ => unreachable!(),
                };

                match kind {
                    ResourceKind::Memory => {
                        resources.push(dev::Resource::Memory(dev::MemoryResource { base, size }));
                    }
                    ResourceKind::Io => {
                        resources.push(dev::Resource::Io(dev::IoResource { base, size }));
                    }
                    ResourceKind::BusNumberRange
                    | ResourceKind::Reserved(_)
                    | ResourceKind::Vendor(_) => {}
                }
            }
            // Extended Interrupt
            0x9 => {
                let info = p.bump()?;

                let mut flags = IrqFlags::empty();
                flags.set(IrqFlags::CONSUMER, info & 0x01 != 0);
                flags.set(IrqFlags::SHARED, info & 0x08 != 0);
                flags.set(IrqFlags::WAKE_CAPABLE, info & 0x10 != 0);

                let trigger = if info & 0x2 > 0 {
                    Trigger::Edge
                } else {
                    Trigger::Level
                };
                let polarity = if info & 0x4 > 0 {
                    Polarity::Low
                } else {
                    Polarity::High
                };

                let num_irqs = p.bump()?;
                for _ in 0..num_irqs {
                    let irq = p.parse_le_u32()?;
                    resources.push(dev::Resource::Irq(dev::IrqResource {
                        irq,
                        flags,
                        trigger,
                        polarity,
                    }));
                }
            }

            _ => {}
        }
    }

    Ok(true)
}

fn parse_resource_descriptors(data: &[u8], resources: &mut Vec<dev::Resource>) {
    let mut p = Parser::new(data);
    while parse_inner(&mut p, resources).unwrap() {}
}

fn probe_node(parent: &Arc<Device>, node: &mut lai::NsNode) {
    let mut resources = vec![];

    if let Some(mut node) = node.resolve_path("_CRS") {
        let var = node.eval().unwrap();
        let Value::Buffer(data) = var.value() else {
            panic!()
        };
        parse_resource_descriptors(data, &mut resources);
    }

    let dev = Arc::new(Device::new(node.name(), resources));
    parent.add_child(&dev);

    for mut child in node
        .children()
        .filter(|node| node.kind() == NodeKind::Device)
    {
        probe_node(&dev, &mut child);
    }

    let id = 'b: {
        for name in ["_HID", "_CID"] {
            if let Some(mut handle) = node.resolve_path(name) {
                match handle.eval().unwrap().value() {
                    Value::String(id) => break 'b id.into(),
                    Value::Integer(pnp_id) => break 'b parse_eisa_id((pnp_id as u32).swap_bytes()),
                    _ => {}
                }
            }
        }
        return;
    };

    println!("ACPI: {id:?}");

    #[cfg(notyet)]
    #[allow(clippy::single_match)]
    match id.as_str() {
        "PNP0103" => dev::hpet::attach(&dev),
        _ => {}
    }
}

fn parse_eisa_id(x: u32) -> String {
    let mut out = [0; 7];

    let parse = |x| 0x40 + (x & 0x1f) as u8;
    out[0] = parse(x >> 26);
    out[1] = parse(x >> 21);
    out[2] = parse(x >> 16);

    let parse = |x| char::from_digit(x & 0xf, 16).unwrap().to_ascii_uppercase() as u8;
    out[3] = parse(x >> 12);
    out[4] = parse(x >> 8);
    out[5] = parse(x >> 4);
    out[6] = parse(x);

    String::from_utf8(Vec::from(out)).unwrap()
}

pub unsafe fn init(dev: &Arc<Device>, rsdp_ptr: *const u8) {
    log::info!("initializing device subsystem");

    let rsdp_ptr = VirtAddr(rsdp_ptr.addr()).to_phys().0 as *const u8;
    let root = RootTable::new(rsdp_ptr, AcpiBridge);

    // LAI will access PCI, so this needs to be done first.
    if let Some(mcfg) = root.get_table::<Mcfg>(0) {
        for entry in mcfg.entries() {
            pcie::initialize_segment_acpi(entry);
        }
    }

    lai::lai_sys::lai_set_acpi_revision(root.acpi_revision as _);

    Lazy::initialize_with(&ACPI_ROOT, RwLock::new(root));

    lai::lai_sys::lai_create_namespace();
    lai::lai_sys::lai_enable_acpi(1);

    let mut system_bus = lai::resolve_path("\\_SB").unwrap();
    probe_node(dev, &mut system_bus);
}
