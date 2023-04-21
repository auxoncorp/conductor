use std::io::{self, BufReader, ErrorKind, Read, Write};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

pub const XAUTHORITY_ENV_VAR: &str = "XAUTHORITY";
pub const DISPLAY_ENV_VAR: &str = "DISPLAY";
pub const HOST_X11_DOMAIN_SOCKET: &str = "/tmp/.X11-unix";

pub(crate) fn system_guest_xauth_file_path<N: AsRef<str>>(system_name: N) -> PathBuf {
    PathBuf::from("/tmp").join(format!(".{}.xauth", system_name.as_ref(),))
}

pub(crate) fn write_guest_xauth<P: AsRef<Path>, W: Write>(
    host_display: &str,
    host_xauthority: P,
    out: &mut W,
) -> io::Result<()> {
    // Convert $DISPLAY into its index, ASCII bytes representation
    let display_number = host_display
        .trim_start_matches(':')
        .parse::<usize>()
        .unwrap();
    let display = display_number.to_string();
    let display = display.as_bytes();

    // Read all of the entries available
    for maybe_entry in XAuthorityEntries::read(host_xauthority)? {
        let mut entry = maybe_entry?;

        // Write out entries for our display (systems with one display don't number entries)
        if entry.number.as_slice() == display || entry.number.is_empty() {
            // Replace source protocol with any/wildcard
            entry.family = AuthEntry::PROTOCOL_WILD;
            entry.write(out)?;
        }
    }

    Ok(())
}

// NOTE: this struct and iterator was taken from
// https://github.com/psychon/x11rb/blob/v0.11.1/x11rb-protocol/src/xauth.rs
/// A single entry of an `.Xauthority` file.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AuthEntry {
    /// The protocol family to which the entry applies
    family: u16,
    /// The address of the peer in a family-specific format
    address: Vec<u8>,
    /// The display number
    number: Vec<u8>,
    /// The name of the authentication method to use for the X11 server described by the previous
    /// fields.
    name: Vec<u8>,
    /// Extra data for the authentication method.
    data: Vec<u8>,
}

impl AuthEntry {
    /// Wildcard matching any protocol family
    const PROTOCOL_WILD: u16 = 0xFFFF;

    fn read<R: Read>(read: &mut R) -> io::Result<Option<Self>> {
        let family = match Self::read_u16(read) {
            Ok(family) => family,
            Err(ref e) if e.kind() == ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        };
        let address = Self::read_string(read)?;
        let number = Self::read_string(read)?;
        let name = Self::read_string(read)?;
        let data = Self::read_string(read)?;
        Ok(Some(AuthEntry {
            family,
            address,
            number,
            name,
            data,
        }))
    }

    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        Self::write_u16(w, self.family)?;
        Self::write_string(w, &self.address)?;
        Self::write_string(w, &self.number)?;
        Self::write_string(w, &self.name)?;
        Self::write_string(w, &self.data)?;
        Ok(())
    }

    fn read_u16<R: Read>(r: &mut R) -> io::Result<u16> {
        let mut buffer = [0; 2];
        r.read_exact(&mut buffer)?;
        Ok(u16::from_be_bytes(buffer))
    }

    fn write_u16<W: Write>(w: &mut W, value: u16) -> io::Result<()> {
        let buffer = value.to_be_bytes();
        w.write_all(&buffer)?;
        Ok(())
    }

    fn read_string<R: Read>(r: &mut R) -> io::Result<Vec<u8>> {
        let length = Self::read_u16(r)?;
        let mut result = vec![0; length.into()];
        r.read_exact(&mut result[..])?;
        Ok(result)
    }

    fn write_string<W: Write>(w: &mut W, value: &[u8]) -> io::Result<()> {
        Self::write_u16(w, value.len() as _)?;
        w.write_all(value)?;
        Ok(())
    }
}

/// An iterator over the entries of an `.Xauthority` file
#[derive(Debug)]
struct XAuthorityEntries(BufReader<File>);

impl XAuthorityEntries {
    fn read<P: AsRef<Path>>(xauthority: P) -> io::Result<XAuthorityEntries> {
        let f = File::open(xauthority)?;
        Ok(XAuthorityEntries(BufReader::new(f)))
    }
}

impl Iterator for XAuthorityEntries {
    type Item = io::Result<AuthEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        AuthEntry::read(&mut self.0).transpose()
    }
}

// TODO - add tests please
