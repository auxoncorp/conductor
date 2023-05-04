use conductor::containers::ContainerStats;
use human_bytes::human_bytes;
use std::io::{self, Write};

#[derive(Clone, Debug)]
pub struct ContainerAndStats {
    pub name: String,
    pub stats: ContainerStats,
}

impl ContainerAndStats {
    pub(crate) const TABWRITER_HEADER: &'static str =
        "NAME\tCPU %\tMEM USAGE / LIMIT\tMEM %\tNET I/O\tBLOCK I/O";

    pub fn new(name: String, stats: ContainerStats) -> Self {
        Self { name, stats }
    }

    pub(crate) fn tabwriter_writeln<W: Write>(&self, w: &mut W) -> Result<(), io::Error> {
        writeln!(
            w,
            "{name}\t{cpu_p:.02}\t{mem_usage} / {mem_limit}\t{mem_p:.02}\t{net_rx} / {net_tx}\t{block_rx} / {block_tx}",
            name = self.name,
            cpu_p = self.stats.cpu_percentage,
            mem_usage = human_bytes(self.stats.mem_usage),
            mem_limit = human_bytes(self.stats.mem_limit),
            mem_p = self.stats.mem_percentage,
            net_rx = human_bytes(self.stats.net_rx),
            net_tx = human_bytes(self.stats.net_tx),
            block_rx = human_bytes(self.stats.block_rx),
            block_tx = human_bytes(self.stats.block_tx),
        )
    }
}
