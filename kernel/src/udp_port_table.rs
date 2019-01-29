//! UDP port table implementation for enforcing port binding. This table is
//! checked when packets are sent/received by a UDPSendStruct/UDPReceiver.
//! UdpPortBinding provides an opaque descriptor object that allows the holder
//! interact with the bound port table. Only the holder of the UdpPortBinding
//! object can interact with its own corresponding location in the bound port
//! table. In order to bind to a particular port as sending/receiving, one must
//! obtain the corresponding sender/receiving binding from UdpPortBinding.
use tock_cells::take_cell::TakeCell;
use core::cell::Cell;
//use capabilities;
use crate::returncode::ReturnCode;
//#![allow(dead_code)]
const MAX_NUM_BOUND_PORTS: usize = 16;
// We need Option<Option<u16>> to distinguish between the case in which we have
// a UdpPortSocket that is not bound to a port and an index where there is no
// UdpPortSocket allocated.
static mut port_table: [Option<Option<u16>>; MAX_NUM_BOUND_PORTS] = [None; MAX_NUM_BOUND_PORTS];

// A UdpPortSocket provides a handle into the bound port table. When binding to
// a port, the socket is consumed and stored inside a UdpPortBinding. When
// undbinding, the socket is returned and can be used to bind to other ports.
pub struct UdpPortSocket {
    idx: usize,
}

// An opaque descriptor object that gives the holder of the object access to
// a particular location (at index idx) of the bound port table.
pub struct UdpPortBinding {
    receive_allocated: Cell<bool>,
    send_allocated: Cell<bool>,
    socket: UdpPortSocket,
    port: u16,
}

// An opaque descriptor that allows the holder to obtain a binding on a port
// for receiving UDP packets.
pub struct UdpReceiverBinding {
    port: u16,
}

// An opaque descriptor that allows the holder to obtain a binding on a port
// for sending UDP packets.
pub struct UdpSenderBinding {
    port: u16,
}

pub struct UdpPortTable {
    port_array: TakeCell<'static, [Option<Option<u16>>]>,
    //max_counter: Cell<usize>,
}

impl UdpPortSocket {
    pub fn new(idx: usize) -> UdpPortSocket {
        UdpPortSocket {idx: idx}
    }
}

impl UdpPortBinding {
    pub fn new(socket: UdpPortSocket, port: u16) -> UdpPortBinding {
        UdpPortBinding {
            receive_allocated: Cell::new(false),
            send_allocated: Cell::new(false),
            socket: socket,
            port: port,
        } // TODO: initialize to what?
    }

    pub fn get_receiver(&self) -> Result<UdpReceiverBinding, ()> {
        // What if self.send_allocated?
        if self.receive_allocated.get() {
           Err(())
        } else {
            self.receive_allocated.set(true);
            Ok(UdpReceiverBinding { port: self.port })
        }
    }

    pub fn put_receiver(&self, recv_binding: UdpReceiverBinding)
        -> Result<(), UdpReceiverBinding> {
        if recv_binding.port == self.port {
            self.receive_allocated.set(false);
            Ok(())
        } else {
            Err(recv_binding)
        }
    }

    pub fn get_sender(&self) -> Result<UdpSenderBinding, ()> {
        if self.send_allocated.get() {
            Err(())
        } else {
            self.send_allocated.set(true);
            Ok(UdpSenderBinding {port: self.port })
        }
    }

    pub fn put_sender(&self, send_binding: UdpSenderBinding)
    -> Result<(), UdpSenderBinding> {
        if send_binding.port == self.port {
            self.send_allocated.set(false);
            Ok(())
        } else {
            Err(send_binding)
        }
    }

    pub fn bound(&self) -> bool {
        self.send_allocated.get() || self.receive_allocated.get()
    }
}

impl UdpSenderBinding {
    pub fn get_port(&self) -> u16 {
        self.port
    }
}

impl UdpReceiverBinding {
    pub fn get_port(&self) -> u16 {
        self.port
    }
}



impl UdpPortTable {
    pub fn new() -> UdpPortTable {
        unsafe {
            UdpPortTable {
                port_array: TakeCell::new(&mut port_table),            }
        }
    }

    pub fn create_socket(&self) -> Result<UdpPortSocket, ReturnCode> {
        self.port_array.map(|table| {
            let mut result: Result<UdpPortSocket, ReturnCode> = Err(ReturnCode::FAIL);
            for i in 0..MAX_NUM_BOUND_PORTS {
                match table[i] {
                    None => {
                        result = Ok(UdpPortSocket::new(i));
                        table[i] = Some(None);
                        break;
                    },
                    _ => (),
                }
            };
            result
        }).unwrap()
    }

    pub fn destroy_socket(&self, socket: UdpPortSocket) {
        self.port_array.map(|table| {
            table[socket.idx] = None;
        });
    }

    // On success, a UdpPortBinding is returned. On failure, the same
    // UdpPortSocket is returned.
    pub fn bind(&self, socket: UdpPortSocket, port: u16,
                /*cap: &capabilities::UDPBindCapability*/) ->
        Result<UdpPortBinding, UdpPortSocket> {
        self.port_array.map(|table| {
            let mut port_exists = false;
            for i in 0..MAX_NUM_BOUND_PORTS {
                match table[i] {
                    Some(Some(p)) => {
                        if (p == port) {
                            port_exists = true;
                        }
                    },
                    _ => (),
                }
            };
            if port_exists {
                Err(socket)
            } else {
                table[socket.idx] = Some(Some(port));
                Ok(UdpPortBinding::new(socket, port))
            }
        }).unwrap()
    }



    // Disassociate the port from the given binding. Return the socket that was
    // contained within the binding object.
    pub fn unbind(&self, binding: UdpPortBinding,
        /*cap: &capabilities::UDPBindCapability*/)
    -> Result<UdpPortSocket, UdpPortBinding> {
        // Need to make sure that the UdpPortBinding itself has no senders
        // or receivers allocated
        if binding.bound() {
            return Err(binding);
        }
        self.port_array.map(|table| {
            table[binding.socket.idx] = None;
        });
        Ok(binding.socket)
    }


}