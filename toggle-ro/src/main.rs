mod error;

const READ_ONLY: u64 = 1u64 << 60;

struct SectorAlignedIO {
    device: std::fs::File,
    off: u64,
    sector_size: usize,
    end_off: u64,
}

impl SectorAlignedIO {
    fn new(
        mut device: std::fs::File,
        sector_size: usize,
    ) -> std::io::Result<Self> {
        let end_off: u64 =
            std::io::Seek::seek(&mut device, std::io::SeekFrom::End(0))?;
        Ok(Self {
            device: device,
            off: 0u64,
            sector_size: sector_size,
            end_off: end_off,
        })
    }
}

impl std::io::Seek for SectorAlignedIO {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.off = match pos {
            std::io::SeekFrom::Start(x) => x,
            std::io::SeekFrom::End(x) => ((self.end_off as i64) + x) as u64,
            std::io::SeekFrom::Current(x) => ((self.off as i64) + x) as u64,
        };
        Ok(self.off)
    }
}

impl std::io::Read for SectorAlignedIO {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let new_off: u64 = self.off + (buf.len() as u64);
        let slop_at_start: usize =
            (self.off % (self.sector_size as u64)) as usize;
        let slop_at_end: usize = self.sector_size
            - ((new_off % (self.sector_size as u64)) as usize)
                % self.sector_size;

        let big_len: usize = buf.len() + slop_at_start + slop_at_end;
        let mut big_buf: Vec<u8> = Vec::with_capacity(big_len);
        big_buf.resize(big_len, 0);

        let _ = std::io::Seek::seek(
            &mut self.device,
            std::io::SeekFrom::Start(self.off - (slop_at_start as u64)),
        )?;
        std::io::Read::read_exact(&mut self.device, &mut big_buf)?;

        buf.copy_from_slice(
            &big_buf[slop_at_start..(big_buf.len() - slop_at_end)],
        );
        self.off = new_off;
        Ok(buf.len())
    }
}

impl std::io::Write for SectorAlignedIO {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let new_off: u64 = self.off + (buf.len() as u64);
        let slop_at_start: usize =
            (self.off % (self.sector_size as u64)) as usize;
        let slop_at_end: usize = self.sector_size
            - ((new_off % (self.sector_size as u64)) as usize)
                % self.sector_size;

        let big_len: usize = buf.len() + slop_at_start + slop_at_end;
        let mut big_buf: Vec<u8> = Vec::with_capacity(big_len);
        big_buf.resize(big_len, 0);

        let _ = std::io::Seek::seek(
            &mut self.device,
            std::io::SeekFrom::Start(self.off - (slop_at_start as u64)),
        )?;
        std::io::Read::read_exact(&mut self.device, &mut big_buf)?;

        let len_minus_slop: usize = big_buf.len() - slop_at_end;
        big_buf[slop_at_start..len_minus_slop].copy_from_slice(buf);
        let _ = std::io::Seek::seek(
            &mut self.device,
            std::io::SeekFrom::Start(self.off - (slop_at_start as u64)),
        )?;
        self.device.write_all(&big_buf)?;
        std::io::Write::flush(&mut self.device)?;
        self.device.sync_all()?;

        self.off = new_off;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn main() -> Result<(), crate::error::Error> {
    let cmdline_parser: clap::Command = clap::Command::new("toggle-ro")
        .color(clap::ColorChoice::Never)
        .arg(
            clap::Arg::new("sector_size")
                .long("sector-size")
                .value_parser(clap::value_parser!(usize))
                .default_value("512"),
        )
        .arg(
            clap::Arg::new("device")
                .value_parser(clap::value_parser!(std::path::PathBuf))
                .required(true),
        );
    let cmdline_matches: clap::ArgMatches = cmdline_parser.get_matches();

    let mut device: SectorAlignedIO = SectorAlignedIO::new(
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(false)
            .create(false)
            .open(
                cmdline_matches
                    .get_one::<std::path::PathBuf>("device")
                    .unwrap(),
            )?,
        *(cmdline_matches.get_one::<usize>("sector_size").unwrap()),
    )?;

    let sector_size_as_u64: u64 = device.sector_size as u64;
    let mut disklabel: gptman::GPT =
        gptman::GPT::read_from(&mut device, sector_size_as_u64)?;

    let mut dirty: bool = false;
    for (index, partition) in disklabel.iter_mut() {
        if partition.is_unused() {
            continue;
        }
        let read_only: bool = (partition.attribute_bits & READ_ONLY) != 0;
        println!("Partition {}: {}", index, partition.partition_name);
        let answer: bool = loop {
            print!("Read only? [{}] ", if read_only { "y" } else { "n" });
            std::io::Write::flush(&mut std::io::stdout().lock())
                .expect("stdout");
            let mut s = String::new();
            std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut s)
                .expect("stdin");
            s = String::from(s.trim());
            if s.is_empty() {
                break read_only;
            } else if s.eq_ignore_ascii_case("y") {
                break true;
            } else if s.eq_ignore_ascii_case("n") {
                break false;
            }
        };

        if answer != read_only {
            if answer {
                partition.attribute_bits |= READ_ONLY;
            } else {
                partition.attribute_bits &= !READ_ONLY;
            }
            dirty = true;
        }

        println!();
    }

    if dirty {
        disklabel.write_into(&mut device)?;
    }
    Ok(())
}
