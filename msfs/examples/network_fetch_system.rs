// Example system that fetches JSON on comm bus command, stores it in a LVar and publishes the body to the comm bus when done.

use msfs::prelude::*;

const EVT_FETCH: &str = "infinity.demo/fetch_config";
const EVT_CONFIG: &str = "infinity.demo/config_bytes";

pub struct NetworkFetchSystem {
    l_last_ok: LVar,
    _sub: Subscription,
}

impl NetworkFetchSystem {
    pub fn new() -> Self {
        let l_last_ok =
            LVar::new("L:INFINITY_FETCH_LAST_OK", "Bool").expect("Failed to create LVar");

        let mut l_for_cb = LVar::new("L:INFINITY_FETCH_LAST_OK", "Bool")
            .expect("Failed to create LVar for callback");

        let sub = Subscription::subscribe(EVT_FETCH, move |_bytes| {
            let params = HttpParams {
                headers: vec![
                    "Accept: application/json".to_string(),
                    "User-Agent: InfinityDemo/1.0".to_string(),
                ],
                post_field: None,
                body: vec![],
            };

            let _ = http_request(
                Method::Get,
                "https://example.com/file.json",
                params,
                move |resp| {
                    let ok = (resp.error_code == 0) as i32;
                    let _ = l_for_cb.set(ok as f64);

                    let _ = commbus_call(
                        EVT_CONFIG,
                        &resp.data,
                        BroadcastFlags::JS | BroadcastFlags::WASM,
                    );
                },
            );
        })
        .expect("subscribe failed");

        Self {
            l_last_ok,
            _sub: sub,
        }
    }
}

impl System for NetworkFetchSystem {
    fn init(&mut self, ctx: &Context, install: &SystemInstall) -> bool {
        let _ = self.l_last_ok.set(0.0);
        true
    }

    fn update(&mut self, ctx: &Context, dt: f32) -> bool {
        true
    }

    fn kill(&mut self, ctx: &Context) -> bool {
        true
    }
}

msfs::export_system!(
    name = network_fetch,
    state = NetworkFetchSystem,
    ctor = NetworkFetchSystem::new()
);
