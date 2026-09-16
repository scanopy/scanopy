//! Emit everything the SNMP simulator's deployment reads, from the typed device definitions.
//!
//! `make snmp-deploy` runs this into a temporary directory and ships what it generated, so the
//! agents on the VM are the structs — there is no committed artifact that can drift from them.
//!
//! Usage: `generate-snmp-fixtures <output-dir> | --credentials | --device-table <path>`
//!   `<output-dir>`        the data files, agent configs and `lab.env` for the deploy tree
//!   `--credentials`       print the credential-seeding SQL to stdout instead
//!   `--device-table <path>` rewrite the device table in that file (`SNMP-TEST-ENV.md`) in place

use std::fs;
use std::path::PathBuf;

use scanopy::daemon::discovery::integration::snmp::sim::{self, emit};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let devices = sim::lab();

    if args.first().is_some_and(|a| a == "--credentials") {
        print!("{}", emit::credentials_sql(&devices));
        return;
    }

    if args.first().is_some_and(|a| a == "--device-table") {
        let Some(path) = args.get(1).map(PathBuf::from) else {
            eprintln!("usage: generate-snmp-fixtures --device-table <path>");
            std::process::exit(2);
        };
        emit::splice_device_table(&path, &devices);
        println!("rewrote the device table in {}", path.display());
        return;
    }

    let Some(out) = args.first().map(PathBuf::from) else {
        eprintln!(
            "usage: generate-snmp-fixtures <output-dir> | --credentials | --device-table <path>"
        );
        std::process::exit(2);
    };

    let data = out.join("data");
    fs::create_dir_all(&data).expect("create output directory");

    let mut files = 0usize;
    let mut confs = 0usize;
    for device in &devices {
        // Variants are written beside the active file and served by nobody until copied over it,
        // which is how a malformed-neighbour shape is swapped in without an snmpd restart.
        for file in device
            .data_files()
            .into_iter()
            .chain(device.variant_files())
            .chain(device.context_files())
        {
            fs::write(data.join(format!("{}.txt", file.name)), file.render())
                .unwrap_or_else(|e| panic!("write {}: {e}", file.name));
            files += 1;
        }

        let conf = out.join(format!("snmpd-{}.conf", device.name));
        fs::write(&conf, emit::snmpd_conf(device)).expect("write config");
        confs += 1;

        if let (Some(context), Some(unit)) =
            (emit::context_conf(device), emit::context_unit(device))
        {
            fs::write(out.join(format!("snmpd-{unit}.conf")), context).expect("write context");
            confs += 1;
        }
    }

    fs::write(out.join("lab.env"), emit::lab_env(&devices)).expect("write lab.env");

    println!(
        "wrote {files} data file(s), {confs} agent config(s) and lab.env to {}",
        out.display()
    );
}
