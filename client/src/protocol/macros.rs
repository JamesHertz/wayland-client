use std::fmt;
/// Syntax in BNF format:
/// ```
/// "declare_interfaces!" "{"
///    "FirstId" "=" <first-interface-id-value> ","
///    (<empty-interface-name>",")* // interfaces with no events or requests
///    ( "@interface" "(" <interface-name> ")" "{"
///        (
///        // TODO: think about getting rid of the []
///          "@requests" "{"
///              (<request-name>"("(<arg> : <type>)*")" ("=>" "[" (<wire-values> ",")+ "]")? ";")+
///          "}"
///        )?
///
///        // An new enum named "<inteface-name>Event" will be generated and for for each <event-name> 
///        // there will be an back on this new enum in CamelCase.
///        "@events" "{" 
///            (<event-name> "(" (<arg> ":" <type>)* ")" ";" )+ 
///        }
///    "}" ",")* 
/// "}"
/// ```
macro_rules! declare_interfaces {
    {  
       FirstId = $start : expr,
       $($skeleton : ident),* 
       $($(,)? @interface($name: ident) $({
            $(@requests {
                $($rt : tt)+
            })?
            $(@events {
                $($et : tt)+
            })?
       })?),* $(,)? 
    } => {
        use $crate::{
            protocol::*,
            wire_format::parsing as parser,
            error
        };

        #[repr(u32)]
        #[allow(clippy::enum_variant_names,non_camel_case_types)] // rust complaining of preffix names
        enum WlIds {
            First = $start,
            $($skeleton,)*
            $($name,)*
        }

        paste::paste! {
            $(
                declare_interfaces!(@decl $name $($(, [< $name Event>], $($et)+ )?)?);
                $(
                    $(declare_interfaces!(@requests $name, $($rt)+);)?
                )?
            )*
        }

        $(declare_interfaces!(@decl $skeleton);)*
    };

    (@requests $name : ident, $($t : tt)+) => {
        impl $name {
            declare_interfaces!(@next_request 0, $($t)+);
        }
    };

    (
        @next_request $id : expr, 
        $request : ident ($($args : tt)*) $(=> [$($expr : expr),+ $(,)? ])?; $($t : tt)*
    ) => {
        pub fn $request (&self, $($args)* ) -> error::Result<usize> {
            let values = &[$($($expr,)+)?];
            log::debug!(
                concat!("{} @ {} -> ", stringify!($request), "({})"),
                self.get_object_id(),
                Self::get_display_name(),
                values.clone().map(|v : WireValue| v.to_string()).join(", ")
            );

            self.0.stream.send(WireMessage {
                object_id: self.get_object_id(),
                request_id: $id,
                values
            })
        }
        declare_interfaces!(@next_request $id + 1, $($t)*);
    };

    (@next_request $id : expr, ) => {}; 
    (@events $t : tt ) => { }; 
    (@events $event_type : ident, 
             $($event_name : ident (
                     $($($arg : ident $t : tt $type : ty),+)?
             ));+ $(;)?
    ) => { 
        paste::paste! {
            #[derive(Debug)]
            pub enum $event_type {
                $([<$event_name:camel>] $(
                    { $($arg : $type),* }
                )?,)+
            }
        }
    };

    (@decl $name : ident, $event_type : ident, $($type_def: tt)+) => {
        declare_interfaces!(@events $event_type , $($type_def)+);

        #[derive(Clone)]
        pub struct $name(WlObjectMetaData); 
        impl WlInterface for $name {

            type Event = $event_type;

            fn get_object_id(&self) -> WaylandId {
                self.0.object_id
            }

            fn get_interface_id() -> WlInterfaceId {
                WlIds::$name as WlInterfaceId
            }

            fn build(object_id: WaylandId, stream: std::rc::Rc<dyn WaylandStream>) -> Self {
                Self(WlObjectMetaData { object_id, stream })
            }

            fn get_display_name() -> &'static str {
                stringify!($name)
            }

            fn parse_event(
                object_id: WaylandId,
                event_id: WlEventId,
                iter: &mut impl Iterator<Item = u8>,
            ) -> Result<Self::Event, WlEventParseError> {
                declare_interfaces!(@next_event object_id, event_id, iter, 0, $($type_def)+);
                Err(WlEventParseError::NoEvent(event_id))
            }
        }
    };

    (@next_event $obj_id : ident, $event_id : ident, $iter: ident, $id : expr, 
            $event_name : ident $(
                ($($arg : ident $t : tt $type : ty),*)
             )?; $($rem : tt)*) => {

        paste::paste! {
            if $event_id == $id {
                $($(
                    let $arg = declare_interfaces!(@parse_arg $type, $iter);
                )*)?

                let args : &[String] = &[$($(format!("{:?}",$arg),)*)?];
                log::debug!(
                    concat!("{} @ {} <- ", stringify!($event_name), "({})"),
                    $obj_id,
                    Self::get_display_name(),
                    args.join(", ")
                );
                return Ok(Self::Event::[<$event_name:camel>] $({$($arg),*})?);
            }
        }

        declare_interfaces!(@next_event $obj_id, $event_id, $iter, $id + 1, $($rem)*);
    };

    (@parse_arg String, $iter : ident) => {
        parser::parse_str($iter)?
    };

    (@parse_arg u32, $iter : ident) => {
        parser::parse_u32($iter)?
    };

    (@parse_arg i32, $iter : ident) => {
        parser::parse_i32($iter)?
    };

    (@parse_arg Array, $iter : ident) => {
        parser::parse_u32_array($iter)?
    };

    (@parse_arg $other : ty, $iter : ident) => {
        compile_error!("Events arguments types should be either 'u32', 'String', 'i32' or 'Vec<u32>'");
    };

    (@next_event $obj_id : ident, $event_id : ident, $iter : ident, $id : expr, ) => { };

    (@decl $name : ident ) => {
        // TODO: remove duplication
        #[derive(Clone)]
        pub struct $name(WlObjectMetaData); 
        impl WlInterface for $name {

            type Event = ();

            fn get_object_id(&self) -> WaylandId {
                self.0.object_id
            }

            fn get_interface_id() -> WlInterfaceId {
                WlIds::$name as WlInterfaceId
            }

            fn build(object_id: WaylandId, stream: std::rc::Rc<dyn WaylandStream>) -> Self {
                Self(WlObjectMetaData { object_id, stream })
            }

            fn get_display_name() -> &'static str {
                stringify!($name)
            }

            fn parse_event(
                object_id: WaylandId,
                event_id: WlEventId,
                iter: &mut impl Iterator<Item = u8>,
            ) -> Result<Self::Event, WlEventParseError> {
                Err(WlEventParseError::NoEvent(event_id))
            }
        }
    };

}

pub(super) use declare_interfaces;
