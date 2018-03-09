//! NRF52 UARTE
//! Universal asynchronous receiver/transmitter with EasyDMA
//!
//! * Author: Niklas Adolfsson <niklasadolfsson1@gmail.com>
//! * Date: July 8, 2017

use core::cell::Cell;
use kernel;
use kernel::common::regs::{ReadOnly, ReadWrite, WriteOnly};
use nrf5x::pinmux;

// this could potentially be replaced to point directly to
// the WRITE_BUFFER in capsules::console::WRITE_BUFFER
const UARTE_BASE: u32 = 0x40002000;
const BUF_SIZE: usize = 64;
static mut BUF: [u8; BUF_SIZE] = [0; BUF_SIZE];

#[repr(C)]
struct UartTeRegisters {
    pub task_startrx: WriteOnly<u32, Task::Register>, // 0x000
    pub task_stoprx: WriteOnly<u32, Task::Register>,  // 0x004
    pub task_starttx: WriteOnly<u32, Task::Register>, // 0x008
    pub task_stoptx: WriteOnly<u32, Task::Register>,  // 0x00c
    _reserved1: [u32; 7],                             // 0x010-0x02c
    pub task_flush_rx: WriteOnly<u32, Task::Register>, // 0x02c
    _reserved2: [u32; 52],                            // 0x030-0x100
    pub event_cts: ReadOnly<u32, Event::Register>,    // 0x100-0x104
    pub event_ncts: ReadOnly<u32, Event::Register>,   // 0x104-0x108
    _reserved3: [u32; 2],                             // 0x108-0x110
    pub event_endrx: ReadOnly<u32, Event::Register>,  // 0x110-0x114
    _reserved4: [u32; 3],                             // 0x114-0x120
    pub event_endtx: ReadOnly<u32, Event::Register>,  // 0x120-0x124
    pub event_error: ReadOnly<u32, Event::Register>,  // 0x124-0x128
    _reserved6: [u32; 7],                             // 0x128-0x144
    pub event_rxto: ReadOnly<u32, Event::Register>,   // 0x144-0x148
    _reserved7: [u32; 1],                             // 0x148-0x14C
    pub event_rxstarted: ReadOnly<u32, Event::Register>, // 0x14C-0x150
    pub event_txstarted: ReadOnly<u32, Event::Register>, // 0x150-0x154
    _reserved8: [u32; 1],                             // 0x154-0x158
    pub event_txstopped: ReadOnly<u32, Event::Register>, // 0x158-0x15c
    _reserved9: [u32; 41],                            // 0x15c-0x200
    pub shorts: ReadWrite<u32, Shorts::Register>,     // 0x200-0x204
    _reserved10: [u32; 64],                           // 0x204-0x304
    pub intenset: ReadWrite<u32, Interrupt::Register>, // 0x304-0x308
    pub intenclr: ReadWrite<u32, Interrupt::Register>, // 0x308-0x30C
    _reserved11: [u32; 93],                           // 0x30C-0x480
    pub errorsrc: ReadWrite<u32, ErrorSrc::Register>, // 0x480-0x484
    _reserved12: [u32; 31],                           // 0x484-0x500
    pub enable: ReadWrite<u32, Enable::Register>,     // 0x500-0x504
    _reserved13: [u32; 1],                            // 0x504-0x508
    pub pselrts: ReadWrite<u32, Psel::Register>,      // 0x508-0x50c
    pub pseltxd: ReadWrite<u32, Psel::Register>,      // 0x50c-0x510
    pub pselcts: ReadWrite<u32, Psel::Register>,      // 0x510-0x514
    pub pselrxd: ReadWrite<u32, Psel::Register>,      // 0x514-0x518
    _reserved14: [u32; 3],                            // 0x518-0x524
    pub baudrate: ReadWrite<u32, Baudrate::Register>, // 0x524-0x528
    _reserved15: [u32; 3],                            // 0x528-0x534
    pub rxd_ptr: ReadWrite<u32, Pointer::Register>,   // 0x534-0x538
    pub rxd_maxcnt: ReadWrite<u32, Counter::Register>, // 0x538-0x53c
    pub rxd_amount: ReadOnly<u32, Counter::Register>, // 0x53c-0x540
    _reserved16: [u32; 1],                            // 0x540-0x544
    pub txd_ptr: ReadWrite<u32, Pointer::Register>,   // 0x544-0x548
    pub txd_maxcnt: ReadWrite<u32, Counter::Register>, // 0x548-0x54c
    pub txd_amount: ReadOnly<u32, Counter::Register>, // 0x54c-0x550
    _reserved17: [u32; 7],                            // 0x550-0x56C
    pub config: ReadWrite<u32, Config::Register>,     // 0x56C-0x570
}

#[cfg_attr(rustfmt, rustfmt_skip)]
register_bitfields! [u32,
    /// Start task
    Task [
        ENABLE OFFSET(0) NUMBITS(1)
    ],

    /// Read event
    Event [
        READY OFFSET(0) NUMBITS(1)
    ],
    
    /// Shortcuts
    Shorts [
        // Shortcut between ENDRX and STARTRX
        ENDRX_STARTRX OFFSET(5) NUMBITS(1),
        // Shortcut between ENDRX and STOPRX
        ENDRX_STOPRX OFFSET(6) NUMBITS(1)
    ],

    /// UART Interrupts
    Interrupt [
        CTS OFFSET(0) NUMBITS(1),
        NCTS OFFSET(1) NUMBITS(1),
        ENDRX OFFSET(4) NUMBITS(1),
        ENDTX OFFSET(8) NUMBITS(1),
        ERROR OFFSET(9) NUMBITS(1),
        RXTO OFFSET(17) NUMBITS(1),
        RXSTARTED OFFSET(19) NUMBITS(1),
        TXSTARTED OFFSET(20) NUMBITS(1),
        TXSTOPPED OFFSET(22) NUMBITS(1)
    ],
    
    /// UART Errors
    ErrorSrc [
        OVERRUN OFFSET(0) NUMBITS(1),
        PARITY OFFSET(1) NUMBITS(1),
        FRAMING OFFSET(2) NUMBITS(1),
        BREAK OFFSET(3) NUMBITS(1)
    ],
    
    /// Enable UART
    Enable [
        ENABLE OFFSET(0) NUMBITS(4) [
           ENABLED = 8,
           DISABLED = 0
        ]
    ],
    
    /// Pin select
    Psel [
        // Pin number
        PIN OFFSET(0) NUMBITS(5),
        // Connect/Disconnect
        CONNECT OFFSET(31) NUMBITS(1)
    ],
    
    /// Baudrate
    Baudrate [
        BAUDRAUTE OFFSET(0) NUMBITS(32)
    ],
    
    /// DMA pointer
    Pointer [
        POINTER OFFSET(0) NUMBITS(32)
    ],
    
    /// Counter value
    Counter [
        COUNTER OFFSET(0) NUMBITS(8)
    ],
    
    /// Configuration of parity and flow control
    Config [
        HWFC OFFSET(0) NUMBITS(1),
        PARITY OFFSET(1) NUMBITS(3)
    ]
];

pub struct UARTE {
    regs: *const UartTeRegisters,
    client: Cell<Option<&'static kernel::hil::uart::Client>>,
    buffer: kernel::common::take_cell::TakeCell<'static, [u8]>,
    remaining_bytes: Cell<usize>,
    offset: Cell<usize>,
}

#[derive(Copy, Clone)]
pub struct UARTParams {
    pub baud_rate: u32,
}

pub static mut UART0: UARTE = UARTE::new();

impl UARTE {
    pub const fn new() -> UARTE {
        UARTE {
            regs: UARTE_BASE as *const UartTeRegisters,
            client: Cell::new(None),
            buffer: kernel::common::take_cell::TakeCell::empty(),
            remaining_bytes: Cell::new(0),
            offset: Cell::new(0),
        }
    }

    pub fn configure(
        &self,
        tx: pinmux::Pinmux,
        rx: pinmux::Pinmux,
        cts: pinmux::Pinmux,
        rts: pinmux::Pinmux,
    ) {
        let regs = unsafe { &*self.regs };
        regs.pseltxd.write(Psel::PIN.val(tx.into()));
        regs.pselrxd.write(Psel::PIN.val(rx.into()));
        regs.pselcts.write(Psel::PIN.val(cts.into()));
        regs.pselrts.write(Psel::PIN.val(rts.into()));
    }

    fn set_baud_rate(&self, baud_rate: u32) {
        let regs = unsafe { &*self.regs };
        match baud_rate {
            1200 => regs.baudrate.set(0x0004F000),
            2400 => regs.baudrate.set(0x0009D000),
            4800 => regs.baudrate.set(0x0013B000),
            9600 => regs.baudrate.set(0x00275000),
            14400 => regs.baudrate.set(0x003AF000),
            19200 => regs.baudrate.set(0x004EA000),
            28800 => regs.baudrate.set(0x0075C000),
            38400 => regs.baudrate.set(0x009D0000),
            57600 => regs.baudrate.set(0x00EB0000),
            76800 => regs.baudrate.set(0x013A9000),
            115200 => regs.baudrate.set(0x01D60000),
            230400 => regs.baudrate.set(0x03B00000),
            250000 => regs.baudrate.set(0x04000000),
            460800 => regs.baudrate.set(0x07400000),
            921600 => regs.baudrate.set(0x0F000000),
            1000000 => regs.baudrate.set(0x10000000),
            _ => regs.baudrate.set(0x01D60000), //setting default to 115200
        }
    }

    fn enable(&self) {
        let regs = unsafe { &*self.regs };
        regs.enable.write(Enable::ENABLE::ENABLED);
    }

    #[allow(dead_code)]
    fn enable_rx_interrupts(&self) {
        let regs = unsafe { &*self.regs };
        regs.intenset.write(Interrupt::ENDRX::SET);
    }

    fn enable_tx_interrupts(&self) {
        let regs = unsafe { &*self.regs };
        regs.intenset.write(Interrupt::ENDTX::SET);
    }

    #[allow(dead_code)]
    fn disable_rx_interrupts(&self) {
        let regs = unsafe { &*self.regs };
        regs.intenclr.write(Interrupt::ENDRX::SET);
    }

    fn disable_tx_interrupts(&self) {
        let regs = unsafe { &*self.regs };
        regs.intenclr.write(Interrupt::ENDTX::SET);
    }

    #[inline(never)]
    // only TX supported here
    pub fn handle_interrupt(&mut self) {
        // disable interrupts
        self.disable_tx_interrupts();

        let regs = unsafe { &*self.regs };

        if regs.event_endtx.matches(Event::READY::SET) {
            regs.task_stoptx.write(Task::ENABLE::SET);
            let tx_bytes = regs.txd_amount.get() as usize;
            let rem = self.remaining_bytes.get();

            // More bytes transmitted than requested
            // Should not happen
            // FIXME: Propogate error to the UART capsule?!
            if tx_bytes > rem {
                debug!("error more bytes than requested\r\n");
                return;
            }

            self.remaining_bytes.set(rem - tx_bytes);
            self.offset.set(tx_bytes);

            if self.remaining_bytes.get() == 0 {
                // Signal client write done
                self.client.get().map(|client| {
                    self.buffer.take().map(|buffer| {
                        client.transmit_complete(buffer, kernel::hil::uart::Error::CommandComplete);
                    });
                });
            }
            // This has been tested however this will only occur if the UART for some reason
            // could not transmit the entire buffer
            else {
                self.set_dma_pointer_to_buffer();
                regs.task_starttx.set(1);
                self.enable_tx_interrupts();
            }
        }
    }

    pub unsafe fn send_byte(&self, byte: u8) {
        let regs = &*self.regs;

        self.remaining_bytes.set(1);
        self.offset.set(0);
        regs.task_stoptx.write(Task::ENABLE::SET);
        BUF[0] = byte;
        self.set_dma_pointer_to_buffer();
        regs.txd_maxcnt.set(1);
        regs.task_starttx.write(Task::ENABLE::SET);

        self.enable_tx_interrupts();
    }

    pub fn tx_ready(&self) -> bool {
        let regs = unsafe { &*self.regs };
        regs.event_endtx.matches(Event::READY::SET)
    }

    fn set_dma_pointer_to_buffer(&self) {
        let regs = unsafe { &*self.regs };
        unsafe { regs.txd_ptr.set(BUF[self.offset.get()..].as_ptr() as u32) }
    }

    fn copy_data_to_uart_buffer(&self, tx_len: usize) {
        self.buffer.map(|buffer| {
            for i in 0..tx_len {
                unsafe { BUF[i] = buffer[i] }
            }
        });
    }
}

impl kernel::hil::uart::UART for UARTE {
    fn set_client(&self, client: &'static kernel::hil::uart::Client) {
        self.client.set(Some(client));
    }

    fn init(&self, params: kernel::hil::uart::UARTParams) {
        self.enable();
        self.set_baud_rate(params.baud_rate);
    }

    fn transmit(&self, tx_data: &'static mut [u8], tx_len: usize) {
        let regs = unsafe { &*self.regs };

        if tx_len == 0 {
            return;
        }

        self.remaining_bytes.set(tx_len);
        self.offset.set(0);
        self.buffer.replace(tx_data);

        // configure dma to point to the the buffer 'BUF'
        self.copy_data_to_uart_buffer(tx_len);

        self.set_dma_pointer_to_buffer();
        // configure length of the buffer to transmit

        regs.txd_maxcnt.set(tx_len as u32);
        regs.task_stoptx.write(Task::ENABLE::SET);
        regs.task_starttx.write(Task::ENABLE::SET);

        self.enable_tx_interrupts();
    }

    #[allow(unused)]
    fn receive(&self, rx_buffer: &'static mut [u8], rx_len: usize) {
        unimplemented!()
    }
}
