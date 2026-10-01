fn main() {
    // Vendored data refreshes live in scripts/refresh-vendored-data.sh
    // (run by `make refresh-vendored-data` and the release workflow).
    println!("cargo:rerun-if-changed=assets/oui.csv");
    println!("cargo:rerun-if-changed=assets/domain-classification");

    // Windows: delay-load the Npcap runtime (packet.dll). pnet links Packet.lib, which makes
    // packet.dll a load-time import, so without Npcap installed an exe fails to even start
    // (0xC0000135). Delay-loading defers the load to the first Packet* call. A failed delay-load
    // raises 0xC06D007E and kills the process, so every Packet* call must sit behind
    // `pnet_datalink::winpcap::packet_dll_available()`. The vendored pnet_datalink does that in
    // `channel()`, and its `interfaces()` makes no Packet* call at all.
    // (wpcap.dll is not imported, so it needs no delay-load entry.) Applied to every linked target,
    // not just the daemon, so Windows test binaries also start on a host without Npcap.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-arg=/DELAYLOAD:packet.dll");
        println!("cargo:rustc-link-arg=delayimp.lib");
    }
}
