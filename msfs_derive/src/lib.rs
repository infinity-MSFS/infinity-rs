use paste::paste;

pub trait AbiTypes {
    type FsContext;
    type SystemInstallData;
    type GaugeInstallData;
    type GaugeDrawData;
}


pub trait SystemModule<A: AbiTypes> {
    fn init(&mut self, ctx: A::FsContext, install: *mut A::SystemInstallData) -> bool;
    fn update(&mut self, ctx: A::FsContext, dt: f32) -> bool;
    fn kill(&mut self, ctx: A::FsContext) -> bool;
}

pub trait GaugeModule<A: AbiTypes> {
    fn init(&mut self, ctx: A::FsContext, install: *mut A::GaugeInstallData) -> bool;
    fn update(&mut self, ctx: A::FsContext, dt: f32) -> bool;
    fn draw(&mut self, ctx: A::FsContext, draw: *mut A::GaugeDrawData) -> bool;
    fn kill(&mut self, ctx: A::FsContext) -> bool;
    fn mouse_handler(&mut self, ctx: A::FsContext, x: f32, y: f32, flags: i32);
}

#[macro_export]
macro_rules! export_system_abi {
    (name=$name:ident, abi=$abi:ty, state=$state:ty, ctor=$ctor:expr, extern=$extern:literal $(,)?) => {
        ::paste::paste! {
            #[allow(non_upper_case_globals)]
            static mut [<$name _SYSTEM_INSTANCE>]: ::core::option::Option<$state> = None;

            #[inline(always)]
            unsafe fn [<$name _system_with>]<R>(f: impl FnOnce(&mut $state) -> R) -> ::core::option::Option<R> {
                [<$name _SYSTEM_INSTANCE>].as_mut().map(f)
            }

            #[no_mangle]
            pub extern $extern fn [<$name _system_init>](
                ctx: < $abi as $crate::AbiTypes >::FsContext,
                p_install: *mut < $abi as $crate::AbiTypes >::SystemInstallData,
            ) -> bool {
                unsafe { [<$name _SYSTEM_INSTANCE>] = Some($ctor); }
                unsafe { [<$name _system_with>](|s| < $state as $crate::SystemModule<$abi> >::init(s, ctx, p_install)).unwrap_or(false) }
            }

            #[no_mangle]
            pub extern $extern fn [<$name _system_update>](
                ctx: < $abi as $crate::AbiTypes >::FsContext,
                d_time: f32,
            ) -> bool {
                unsafe { [<$name _system_with>](|s| < $state as $crate::SystemModule<$abi> >::update(s, ctx, d_time)).unwrap_or(false) }
            }

            #[no_mangle]
            pub extern $extern fn [<$name _system_kill>](
                ctx: < $abi as $crate::AbiTypes >::FsContext,
            ) -> bool {
                unsafe {
                    let ok = [<$name _system_with>](|s| < $state as $crate::SystemModule<$abi> >::kill(s, ctx)).unwrap_or(false);
                    [<$name _SYSTEM_INSTANCE>] = None;
                    ok
                }
            }
        }
    };

    (name=$name:ident, abi=$abi:ty, state=$state:ty, ctor=$ctor:expr $(,)?) => {
        $crate::export_system_abi!(name=$name, abi=$abi, state=$state, ctor=$ctor, extern="C");
    };
}

#[macro_export]
macro_rules! export_gauge_abi {
    (name=$name:ident, abi=$abi:ty, state=$state:ty, ctor=$ctor:expr, extern=$extern:literal $(,)?) => {
        ::paste::paste! {
            #[allow(non_upper_case_globals)]
            static mut [<$name _GAUGE_INSTANCE>]: ::core::option::Option<$state> = None;

            #[inline(always)]
            unsafe fn [<$name _gauge_with>]<R>(f: impl FnOnce(&mut $state) -> R) -> ::core::option::Option<R> {
                [<$name _GAUGE_INSTANCE>].as_mut().map(f)
            }

            #[no_mangle]
            pub extern $extern fn [<$name _gauge_init>](
                ctx: < $abi as $crate::AbiTypes >::FsContext,
                p_install: *mut < $abi as $crate::AbiTypes >::GaugeInstallData,
            ) -> bool {
                unsafe { [<$name _GAUGE_INSTANCE>] = Some($ctor); }
                unsafe { [<$name _gauge_with>](|g| < $state as $crate::GaugeModule<$abi> >::init(g, ctx, p_install)).unwrap_or(false) }
            }

            #[no_mangle]
            pub extern $extern fn [<$name _gauge_update>](
                ctx: < $abi as $crate::AbiTypes >::FsContext,
                d_time: f32,
            ) -> bool {
                unsafe { [<$name _gauge_with>](|g| < $state as $crate::GaugeModule<$abi> >::update(g, ctx, d_time)).unwrap_or(false) }
            }

            #[no_mangle]
            pub extern $extern fn [<$name _gauge_draw>](
                ctx: < $abi as $crate::AbiTypes >::FsContext,
                p_draw: *mut < $abi as $crate::AbiTypes >::GaugeDrawData,
            ) -> bool {
                unsafe { [<$name _gauge_with>](|g| < $state as $crate::GaugeModule<$abi> >::draw(g, ctx, p_draw)).unwrap_or(false) }
            }

            #[no_mangle]
            pub extern $extern fn [<$name _gauge_kill>](
                ctx: < $abi as $crate::AbiTypes >::FsContext,
            ) -> bool {
                unsafe {
                    let ok = [<$name _gauge_with>](|g| < $state as $crate::GaugeModule<$abi> >::kill(g, ctx)).unwrap_or(false);
                    [<$name _GAUGE_INSTANCE>] = None;
                    ok
                }
            }

            #[no_mangle]
            pub extern $extern fn [<$name _gauge_mouse_handler>](
                ctx: < $abi as $crate::AbiTypes >::FsContext,
                f_x: f32,
                f_y: f32,
                i_flags: i32,
            ) {
                unsafe {
                    let _ = [<$name _gauge_with>](|g| {
                        < $state as $crate::GaugeModule<$abi> >::mouse_handler(g, ctx, f_x, f_y, i_flags);
                    });
                }
            }
        }
    };

    (name=$name:ident, abi=$abi:ty, state=$state:ty, ctor=$ctor:expr $(,)?) => {
        $crate::export_gauge_abi!(name=$name, abi=$abi, state=$state, ctor=$ctor, extern="C");
    };
}