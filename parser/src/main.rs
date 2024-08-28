//   <interface name="wl_display" version="1">
//     <description summary="core global object">
//       The core global object.  This is a special singleton object.  It
//       is used for internal Wayland protocol features.
//     </description>
//
//     <request name="sync">
//       <description summary="asynchronous roundtrip">
// The sync request asks the server to emit the 'done' event
// on the returned wl_callback object.  Since requests are
// handled in-order and events are delivered in-order, this can
// be used as a barrier to ensure all previous requests and the
// resulting events have been handled.
//
// The object returned by this request will be destroyed by the
// compositor after the callback is fired and as such the client must not
// attempt to use it after that point.
//
// The callback_data passed in the callback is undefined and should be ignored.
//       </description>
//       <arg name="callback" type="new_id" interface="wl_callback"
//    summary="callback object for the sync request"/>
//     </request>
//
//     <request name="get_registry">
//       <description summary="get global registry object">
// This request creates a registry object that allows the client
// to list and bind the global objects available from the
// compositor.
//
// It should be noted that the server side resources consumed in
// response to a get_registry request can only be released when the
// client disconnects, not when the client side proxy is destroyed.
// Therefore, clients should invoke get_registry as infrequently as
// possible to avoid wasting memory.
//       </description>
//       <arg name="registry" type="new_id" interface="wl_registry"
//    summary="global registry object"/>
//     </request>
//
//     <event name="error">
//       <description summary="fatal error event">
// The error event is sent out when a fatal (non-recoverable)
// error has occurred.  The object_id argument is the object
// where the error occurred, most often in response to a request
// to that object.  The code identifies the error and is defined
// by the object interface.  As such, each interface defines its
// own set of error codes.  The message is a brief description
// of the error, for (debugging) convenience.
//       </description>
//       <arg name="object_id" type="object" summary="object where the error occurred"/>
//       <arg name="code" type="uint" summary="error code"/>
//       <arg name="message" type="string" summary="error description"/>
//     </event>
//
//     <enum name="error">
//       <description summary="global error values">
// These errors are global and can be emitted in response to any
// server request.
//       </description>
//       <entry name="invalid_object" value="0"
//      summary="server couldn't find object"/>
//       <entry name="invalid_method" value="1"
//      summary="method doesn't exist on the specified interface or malformed request"/>
//       <entry name="no_memory" value="2"
//      summary="server is out of memory"/>
//       <entry name="implementation" value="3"
//      summary="implementation error in compositor"/>
//     </enum>
//
//     <event name="delete_id">
//       <description summary="acknowledge object ID deletion">
// This event is used internally by the object ID management
// logic. When a client deletes an object that it had created,
// the server will send this event to acknowledge that it has
// seen the delete request. When the client receives this event,
// it will know that it can safely reuse the object ID.
//       </description>
//       <arg name="id" type="uint" summary="deleted object ID"/>
//     </event>
//   </interface>

//   <interface name="wl_registry" version="1">
//     <description summary="global registry object">
//       The singleton global registry object.  The server has a number of
//       global objects that are available to all clients.  These objects
//       typically represent an actual object in the server (for example,
//       an input device) or they are singleton objects that provide
//       extension functionality.
//
//       When a client creates a registry object, the registry object
//       will emit a global event for each global currently in the
//       registry.  Globals come and go as a result of device or
//       monitor hotplugs, reconfiguration or other events, and the
//       registry will send out global and global_remove events to
//       keep the client up to date with the changes.  To mark the end
//       of the initial burst of events, the client can use the
//       wl_display.sync request immediately after calling
//       wl_display.get_registry.
//
//       A client can bind to a global object by using the bind
//       request.  This creates a client-side handle that lets the object
//       emit events to the client and lets the client invoke requests on
//       the object.
//     </description>
//
//     <request name="bind">
//       <description summary="bind an object to the display">
// Binds a new, client-created object to the server using the
// specified name as the identifier.
//       </description>
//       <arg name="name" type="uint" summary="unique numeric name of the object"/>
//       <arg name="id" type="new_id" summary="bounded object"/>
//     </request>
//
//     <event name="global">
//       <description summary="announce global object">
// Notify the client of global objects.
//
// The event notifies the client that a global object with
// the given name is now available, and it implements the
// given version of the given interface.
//       </description>
//       <arg name="name" type="uint" summary="numeric name of the global object"/>
//       <arg name="interface" type="string" summary="interface implemented by the object"/>
//       <arg name="version" type="uint" summary="interface version"/>
//     </event>
//
//     <event name="global_remove">
//       <description summary="announce removal of global object">
// Notify the client of removed global objects.
//
// This event notifies the client that the global identified
// by name is no longer available.  If the client bound to
// the global using the bind request, the client should now
// destroy that object.
//
// The object remains valid and requests to the object will be
// ignored until the client destroys it, to avoid races between
// the global going away and a client sending a request to it.
//       </description>
//       <arg name="name" type="uint" summary="numeric name of the global object"/>
//     </event>
//   </interface>
//

use self::WireValue::*;
use std::rc::Rc;

// type NewId = u32;
type WaylandId = u32;
type Result<T> = std::result::Result<T, std::io::Error>;

#[derive(Debug)]
enum WireValue {
    Uint32(u32),
    Int32(u32),
    Str(String),
    Array(Vec<u8>),
}

#[derive(Debug)]
struct WireMessage<'a> {
    interface_id: WlInterfaceId,
    object_id: WaylandId,
    request_id: WaylandId,
    values: &'a [WireValue],
}

// Implementation of this trait are recommended to use interior mutability
// (https://doc.rust-lang.org/reference/interior-mutability.html)
trait WaylandStream {
    fn send(&self, msg: WireMessage) -> Result<usize>;
}

trait WaylandInterface {
    fn get_interface_id() -> WlInterfaceId;
    // where Self: Sized;
    fn build(object_id: WaylandId, stream: Rc<dyn WaylandStream>) -> Self;
    // where Self: Sized;
    fn get_object_id(&self) -> WaylandId;
    fn get_event(&self, event_id: WaylandId, payload: &[u8]) -> Result<WlEvent>;
}

struct WlObjectMetaData {
    object_id: WaylandId,
    stream: Rc<dyn WaylandStream>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
enum WlInterfaceId {
    WlDisplay,
    // WlRegistry
}

// #[derive(Debug, PartialEq, Eq)]
enum WlEvent {
    // WlDisplayError { object : WaylandId, code : u32, message : String }
}

struct WlDisplay(WlObjectMetaData);

impl WlDisplay {
    fn sync(&self, new_id: WaylandId) -> Result<usize> {
        self.0.stream.send(WireMessage {
            interface_id: Self::get_interface_id(),
            object_id: self.0.object_id,
            request_id: 1,
            values: &[Uint32(new_id)],
        })
    }
}

impl WaylandInterface for WlDisplay {
    fn get_interface_id() -> WlInterfaceId {
        WlInterfaceId::WlDisplay
    }

    fn build(object_id: u32, stream: Rc<dyn WaylandStream>) -> Self {
        Self(WlObjectMetaData { object_id, stream })
    }

    fn get_object_id(&self) -> WaylandId {
        self.0.object_id
    }

    fn get_event(
        &self,
        _event_id: WaylandId,
        _payload: &[u8],
    ) -> Result<WlEvent> {
        todo!()
    }
}
//
// struct JockingStream(RefCell<u32>);
// impl WaylandStream for JockingStream {
//     fn send(&self, msg : WireMessage) -> Result<usize> {
//         println!("Received yet another message {msg:?}");
//         let mut value = self.0.borrow_mut();
//         *value += 1;
//         Ok(0)
//     }
// }
//
// impl JockingStream {
//     fn new() -> Self {
//         Self (RefCell::new(0))
//     }
//
//     fn current_value(&self) -> u32 {
//         *self.0.borrow()
//     }
// }
//
//
// struct Client(Rc<JockingStream>);
// impl Client {
//
//     fn get_global <T: WaylandInterface>(&self) -> T {
//         T::build(0, self.0.clone())
//     }
// }
//
// to handle events use this later: https://github.com/Robbepop/enum-tag
fn main() -> Result<()> {
    // let client = Client(
    //     Rc::new(JockingStream::new())
    // );
    //
    // let display : WlDisplay = client.get_global();
    // // display.get_object_id
    // // display.sync(10)?;
    // // display.sync(10)?;
    // // display.sync(10)?;
    //
    // println!("value {}", client.0.current_value());
    //
    Ok(())
}
