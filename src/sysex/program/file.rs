//! File format support for `.mnlgxdprog` and `.mnlgxdlib` files.
//!
//! These files are zip archives containing raw program data blobs. This module
//! is feature-gated behind `file-formats`.
//!
//! - `.mnlgxdprog`: a zip containing `Prog_000.prog_bin` (1024 bytes).
//! - `.mnlgxdlib`: a zip containing `Prog_000.prog_bin` through
//!   `Prog_NNN.prog_bin` plus `FileInformation.xml`.

use std::io::{Read, Seek, Write};

use zip::write::SimpleFileOptions;
use zip::ZipArchive;
use zip::ZipWriter;

use crate::error::{Error, Result};
use crate::sysex::program::ProgramData;

// ---------------------------------------------------------------------------
// .mnlgxdprog (single program)
// ---------------------------------------------------------------------------

/// Read a `.mnlgxdprog` file (zip archive with `Prog_000.prog_bin`).
///
/// # Errors
///
/// Returns an error if the archive cannot be read, is missing the expected
/// entry, or the program data is invalid.
pub fn read_prog_file<R: Read + Seek>(reader: R) -> Result<ProgramData> {
    let mut archive =
        ZipArchive::new(reader).map_err(|e| Error::Zip(format!("failed to open archive: {e}")))?;

    let mut prog_file = archive
        .by_name("Prog_000.prog_bin")
        .map_err(|e| Error::Zip(format!("missing Prog_000.prog_bin: {e}")))?;

    let mut buf = Vec::with_capacity(ProgramData::SIZE);
    prog_file
        .read_to_end(&mut buf)
        .map_err(|e| Error::Zip(format!("failed to read prog_bin: {e}")))?;

    ProgramData::from_bytes(&buf)
}

/// Write a `.mnlgxdprog` file (zip archive with `Prog_000.prog_bin`).
///
/// # Errors
///
/// Returns an error if writing to the zip archive fails.
pub fn write_prog_file<W: Write + Seek>(writer: W, data: &ProgramData) -> Result<()> {
    let mut zip = ZipWriter::new(writer);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("Prog_000.prog_bin", options)
        .map_err(|e| Error::Zip(format!("failed to create entry: {e}")))?;

    let raw = data.to_bytes();
    zip.write_all(&raw)?;

    zip.finish()
        .map_err(|e| Error::Zip(format!("failed to finalize archive: {e}")))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// .mnlgxdlib (program library)
// ---------------------------------------------------------------------------

/// Read a `.mnlgxdlib` file (zip archive with multiple `Prog_NNN.prog_bin`).
///
/// Returns a vector of programs in order of their file names.
///
/// # Errors
///
/// Returns an error if the archive cannot be read or any program is invalid.
pub fn read_lib_file<R: Read + Seek>(reader: R) -> Result<Vec<ProgramData>> {
    let mut archive =
        ZipArchive::new(reader).map_err(|e| Error::Zip(format!("failed to open archive: {e}")))?;

    // Collect program file names and sort them.
    let mut prog_names: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| Error::Zip(format!("failed to read entry {i}: {e}")))?;
        let name = file.name().to_string();
        if name.ends_with(".prog_bin") {
            prog_names.push(name);
        }
    }
    prog_names.sort();

    let mut programs = Vec::with_capacity(prog_names.len());
    for name in &prog_names {
        let mut file = archive
            .by_name(name)
            .map_err(|e| Error::Zip(format!("failed to read {name}: {e}")))?;

        let mut buf = Vec::with_capacity(ProgramData::SIZE);
        file.read_to_end(&mut buf)
            .map_err(|e| Error::Zip(format!("failed to read {name}: {e}")))?;

        programs.push(ProgramData::from_bytes(&buf)?);
    }

    Ok(programs)
}

/// Write a `.mnlgxdlib` file (zip archive with multiple programs).
///
/// Programs are written as `Prog_000.prog_bin`, `Prog_001.prog_bin`, etc.
/// A minimal `FileInformation.xml` is also included.
///
/// # Errors
///
/// Returns an error if writing to the zip archive fails.
pub fn write_lib_file<W: Write + Seek>(writer: W, programs: &[ProgramData]) -> Result<()> {
    let mut zip = ZipWriter::new(writer);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for (i, prog) in programs.iter().enumerate() {
        let name = format!("Prog_{i:03}.prog_bin");
        zip.start_file(&name, options)
            .map_err(|e| Error::Zip(format!("failed to create entry {name}: {e}")))?;
        let raw = prog.to_bytes();
        zip.write_all(&raw)?;
    }

    // Write a minimal FileInformation.xml.
    zip.start_file("FileInformation.xml", options)
        .map_err(|e| Error::Zip(format!("failed to create FileInformation.xml: {e}")))?;
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <KorgMSLibrarian_Data>\n\
         <Product>minilogue xd</Product>\n\
         <Contents NumProgramData=\"{}\"/>\n\
         </KorgMSLibrarian_Data>\n",
        programs.len()
    );
    zip.write_all(xml.as_bytes())?;

    zip.finish()
        .map_err(|e| Error::Zip(format!("failed to finalize archive: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sysex::program::ProgramName;
    use std::io::Cursor;

    #[test]
    fn prog_file_round_trip() {
        let mut data = ProgramData::default();
        data.synth.name = ProgramName::from_string("FileTest").unwrap();
        data.synth.cutoff = 999;

        let mut buf = Cursor::new(Vec::new());
        write_prog_file(&mut buf, &data).unwrap();

        buf.set_position(0);
        let recovered = read_prog_file(&mut buf).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn prog_file_preserves_all_fields() {
        let mut data = ProgramData::default();
        data.synth.delay_on = true;
        data.synth.reverb_on = true;
        data.synth.mod_fx_on = true;
        data.sequencer.bpm = 2400;
        data.sequencer.steps[0].notes[0] = 60;
        data.sequencer.steps[0].velocities[0] = 127;

        let mut buf = Cursor::new(Vec::new());
        write_prog_file(&mut buf, &data).unwrap();

        buf.set_position(0);
        let recovered = read_prog_file(&mut buf).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn lib_file_round_trip() {
        let mut programs = Vec::new();
        for i in 0..3 {
            let mut data = ProgramData::default();
            data.synth.name = ProgramName::from_string(&format!("Prog {i}")).unwrap();
            data.synth.cutoff = i as u16 * 100;
            programs.push(data);
        }

        let mut buf = Cursor::new(Vec::new());
        write_lib_file(&mut buf, &programs).unwrap();

        buf.set_position(0);
        let recovered = read_lib_file(&mut buf).unwrap();
        assert_eq!(recovered.len(), 3);
        for (i, prog) in recovered.iter().enumerate() {
            assert_eq!(prog.synth.name.as_str(), format!("Prog {i}"));
            assert_eq!(prog.synth.cutoff, i as u16 * 100);
        }
    }

    #[test]
    fn lib_file_empty() {
        let programs: Vec<ProgramData> = Vec::new();
        let mut buf = Cursor::new(Vec::new());
        write_lib_file(&mut buf, &programs).unwrap();

        buf.set_position(0);
        let recovered = read_lib_file(&mut buf).unwrap();
        assert!(recovered.is_empty());
    }

    #[test]
    fn lib_file_single_program() {
        let data = ProgramData::default();
        let mut buf = Cursor::new(Vec::new());
        write_lib_file(&mut buf, &[data.clone()]).unwrap();

        buf.set_position(0);
        let recovered = read_lib_file(&mut buf).unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0], data);
    }

    #[test]
    fn read_prog_file_missing_entry() {
        // Create a zip without Prog_000.prog_bin.
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buf);
            let options = SimpleFileOptions::default();
            zip.start_file("other.txt", options).unwrap();
            zip.write_all(b"hello").unwrap();
            zip.finish().unwrap();
        }

        buf.set_position(0);
        assert!(read_prog_file(&mut buf).is_err());
    }

    #[test]
    fn read_prog_file_invalid_data() {
        // Create a zip with Prog_000.prog_bin that is too short.
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buf);
            let options = SimpleFileOptions::default();
            zip.start_file("Prog_000.prog_bin", options).unwrap();
            zip.write_all(&[0u8; 100]).unwrap();
            zip.finish().unwrap();
        }

        buf.set_position(0);
        assert!(read_prog_file(&mut buf).is_err());
    }
}
