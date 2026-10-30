// SPDX-License-Identifier: MIT
//! Qualified NOR acquisition/install/restore. All checks precede the first erase.
use crate::Client;
use anyhow::{Context, Result, ensure};
use apdo_protocol as wire;
use km003c_backup as backup;
use std::{fs, path::Path};
const NOR_BYTES: usize = 0x400000;
const PREFIX: usize = wire::GAME_LIMIT as usize;
const PACK_BYTES: usize = 2802000;
const PACK_SHA: &str = "6b25a4f1adb6f1f1f5f42226c272b93ccff4f356ac8d7034cf83712b2da70c08";

trait Memory {
    fn read(&mut self, at: u32, n: usize) -> Result<Vec<u8>>;
    fn erase(&mut self, at: u32) -> Result<()>;
    fn write(&mut self, at: u32, data: &[u8]) -> Result<()>;
}
impl Memory for Client {
    fn read(&mut self, at: u32, n: usize) -> Result<Vec<u8>> {
        ensure!(
            at.checked_add(n.try_into()?)
                .is_some_and(|end| end <= NOR_BYTES as u32),
            "NOR read range"
        );
        let mut bytes = Vec::with_capacity(n);
        while bytes.len() < n {
            let count = (n - bytes.len()).min(wire::MAX_DATA);
            let (_, data) = self.call(wire::NOR_READ, at + bytes.len() as u32, count as u32)?;
            ensure!(data.len() == count, "Short NOR read");
            bytes.extend(data);
        }
        Ok(bytes)
    }
    fn erase(&mut self, at: u32) -> Result<()> {
        self.call(wire::NOR_ERASE, at, wire::ERASE_CONFIRM)?;
        Ok(())
    }
    fn write(&mut self, at: u32, data: &[u8]) -> Result<()> {
        ensure!(wire::writable(at, data.len() as u32), "NOR page boundary");
        let mut req = wire::request(wire::NOR_WRITE, 0, at, data.len() as u32);
        req[16..16 + data.len()].copy_from_slice(data);
        self.exchange(req)?;
        Ok(())
    }
}
fn original(dir: &Path, stock: &Path) -> Result<Vec<u8>> {
    backup::verify_qualified(stock)?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("verified.json"))?)?;
    let bytes = fs::read(dir.join("external-4MiB.bin"))?;
    ensure!(
        bytes.len() == NOR_BYTES
            && manifest["schema_version"] == 1
            && manifest["matching_reads"] == 2,
        "Need an independent double-read 4 MiB NOR backup"
    );
    ensure!(
        manifest["stock_manifest_sha256"]
            == backup::digest(&fs::read(stock.join("manifest.json"))?)
            && manifest["sha256"] == backup::digest(&bytes),
        "NOR backup qualification or hash mismatch"
    );
    ensure!(
        bytes[..PREFIX] == fs::read(stock.join(backup::PREFIX))?,
        "NOR backup prefix differs from qualified stock acquisition"
    );
    Ok(bytes)
}
pub fn acquire(stock: &Path, out: &Path) -> Result<()> {
    let qualified = backup::verify_qualified(stock)?;
    let prefix = fs::read(stock.join(backup::PREFIX))?;
    let mut client = Client::open_on(Some(&qualified.usb_port))?;
    client.call(wire::PAUSE, 0, 0)?;
    fs::create_dir(out).context("NOR backup directory must be new")?;
    let first = client.read(0, NOR_BYTES)?;
    ensure!(
        first[..PREFIX] == prefix,
        "Meter no longer contains the qualified stock prefix; no original backup can be acquired here"
    );
    let second = client.read(0, NOR_BYTES)?;
    ensure!(first == second, "NOR double reads differ");
    backup::save(&out.join("external-4MiB.bin"), &first)?;
    backup::save_json(
        &out.join("verified.json"),
        &serde_json::json!({
        "schema_version":1,"matching_reads":2,"bytes":NOR_BYTES,
        "stock_manifest_sha256":backup::digest(&fs::read(stock.join("manifest.json"))?),
        "sha256":backup::digest(&first),"protected_tail_sha256":backup::digest(&first[PREFIX..])}),
    )?;
    original(out, stock)?;
    println!("Qualified full original NOR backup saved");
    Ok(())
}
fn planned(original: &[u8], pack: &[u8]) -> Result<Vec<u8>> {
    ensure!(
        original.len() == NOR_BYTES,
        "Need complete NOR recovery data"
    );
    ensure!(
        pack.len() == PACK_BYTES && backup::digest(pack) == PACK_SHA,
        "Use the exact pinned Freedoom E1M1 pack"
    );
    let mut expected = original.to_vec();
    expected[..pack.len()].copy_from_slice(pack);
    Ok(expected)
}
fn preflight(current: &[u8], stock: &[u8], game: &[u8]) -> Result<()> {
    ensure!(
        current.len() == NOR_BYTES && stock.len() == NOR_BYTES && game.len() == NOR_BYTES,
        "Incomplete NOR data"
    );
    ensure!(
        current[PREFIX..] == stock[PREFIX..],
        "Protected NOR tail differs; stop and investigate"
    );
    ensure!(
        current[..PREFIX] == stock[..PREFIX] || current[..PREFIX] == game[..PREFIX],
        "NOR prefix is neither the qualified stock data nor the exact planned game image"
    );
    Ok(())
}
fn program(memory: &mut impl Memory, current: &[u8], target: &[u8]) -> Result<usize> {
    ensure!(
        current.len() == NOR_BYTES
            && target.len() == NOR_BYTES
            && current[PREFIX..] == target[PREFIX..],
        "Protected tail must remain unchanged"
    );
    let mut count = 0;
    for sector in (0..PREFIX).step_by(4096) {
        if current[sector..sector + 4096] == target[sector..sector + 4096] {
            continue;
        }
        memory.erase(sector as u32)?;
        count += 1;
        for page in (sector..sector + 4096).step_by(256) {
            for offset in (0..256).step_by(48) {
                let n = 48.min(256 - offset);
                let at = page + offset;
                let bytes = &target[at..at + n];
                if bytes.iter().any(|b| *b != 255) {
                    memory.write(at as u32, bytes)?;
                }
            }
        }
        if count % 32 == 0 {
            eprintln!("Verified plan: programmed {count} sectors");
        }
    }
    Ok(count)
}
pub fn install(
    stock: &Path,
    nor: &Path,
    pack: &Path,
    evidence: &Path,
    restore: bool,
    dry_run: bool,
) -> Result<()> {
    let qualified = backup::verify_qualified(stock)?;
    let original = original(nor, stock)?;
    let game = planned(&original, &fs::read(pack)?)?;
    let target = if restore { &original } else { &game };
    let mut client = Client::open_on(Some(&qualified.usb_port))?;
    client.call(wire::PAUSE, 0, 0)?;
    let state = client.info()?;
    ensure!(
        state[0] == 0 && state[1] == 0,
        "Hold OK during app startup to enter storage service before initializing DOOM"
    );
    let current = client.read(0, NOR_BYTES)?;
    preflight(&current, &original, &game)?;
    fs::create_dir(evidence).context("Evidence directory must be new")?;
    backup::save_json(
        &evidence.join("before.json"),
        &serde_json::json!({
        "restore":restore,"dry_run":dry_run,"current_sha256":backup::digest(&current),
        "target_sha256":backup::digest(target),"protected_tail_sha256":backup::digest(&original[PREFIX..])}),
    )?;
    if dry_run {
        println!("Preflight passed; dry run performed no erase/program commands");
        return Ok(());
    }
    let sectors = program(&mut client, &current, target)?;
    let actual = client.read(0, NOR_BYTES)?;
    backup::save(&evidence.join("external-readback.bin"), &actual)?;
    ensure!(
        actual == *target,
        "Full NOR readback differs; do not start or restore the stock app"
    );
    backup::save_json(
        &evidence.join("verified.json"),
        &serde_json::json!({
        "readback_verified":true,"protected_tail_unchanged":true,"sectors_programmed":sectors,
        "sha256":backup::digest(&actual)}),
    )?;
    println!("Full 4 MiB readback verified; protected tail unchanged; app remains paused");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        bytes: Vec<u8>,
        writes: usize,
        erases: usize,
    }
    impl Memory for Fake {
        fn read(&mut self, at: u32, n: usize) -> Result<Vec<u8>> {
            Ok(self.bytes[at as usize..at as usize + n].to_vec())
        }
        fn erase(&mut self, at: u32) -> Result<()> {
            assert!(wire::erasable(at));
            self.erases += 1;
            self.bytes[at as usize..at as usize + 4096].fill(255);
            Ok(())
        }
        fn write(&mut self, at: u32, data: &[u8]) -> Result<()> {
            assert!(wire::writable(at, data.len() as u32));
            self.writes += 1;
            for (a, b) in self.bytes[at as usize..].iter_mut().zip(data) {
                *a &= *b;
            }
            Ok(())
        }
    }
    #[test]
    fn unknown_prefix_and_changed_tail_fail_before_programming() {
        let stock = vec![17; NOR_BYTES];
        let mut game = stock.clone();
        game[..4096].fill(23);
        let mut unknown = stock.clone();
        unknown[123] = 99;
        assert!(preflight(&unknown, &stock, &game).is_err());
        unknown = game.clone();
        unknown[PREFIX + 4] ^= 1;
        assert!(preflight(&unknown, &stock, &game).is_err());
        assert!(preflight(&stock, &stock, &game).is_ok());
        assert!(preflight(&game, &stock, &game).is_ok());
    }
    #[test]
    fn install_and_restore_preserve_every_other_byte_and_skip_identical_sectors() {
        let stock = vec![17; NOR_BYTES];
        let mut game = stock.clone();
        game[13..1089].fill(23);
        let mut fake = Fake {
            bytes: stock.clone(),
            writes: 0,
            erases: 0,
        };
        assert_eq!(program(&mut fake, &stock, &game).unwrap(), 1);
        assert_eq!(fake.bytes, game);
        assert_eq!(program(&mut fake, &game, &game).unwrap(), 0);
        assert_eq!(fake.erases, 1);
        program(&mut fake, &game, &stock).unwrap();
        assert_eq!(fake.bytes, stock);
    }
}
