use std::collections::HashMap;
use std::path::PathBuf;

use num_rational::Ratio;
use psutil::disk;
use tui::buffer::Buffer;
use tui::layout::{Constraint, Rect};
use tui::style::Modifier;
use tui::widgets::{Row, Table, Widget};

use crate::colorscheme::Colorscheme;
use crate::update::UpdatableWidget;
use crate::widgets::block;

#[derive(Clone)]
struct Partition {
	name: String,
	mountpoint: PathBuf,
	bytes_read: u64,
	bytes_written: u64,
	bytes_read_recently: u64,
	bytes_written_recently: u64,
	used_percent: f32,
	bytes_free: u64,
}

pub struct DiskWidget<'a> {
	title: String,
	update_interval: Ratio<u64>,
	colorscheme: &'a Colorscheme,

	partitions: HashMap<String, Partition>,

	collector: disk::DiskIoCountersCollector,
}

impl DiskWidget<'_> {
	pub fn new(colorscheme: &Colorscheme) -> DiskWidget {
		DiskWidget {
			title: " Disk Usage ".to_string(),
			update_interval: Ratio::from_integer(1),
			colorscheme,

			partitions: HashMap::new(),

			collector: disk::DiskIoCountersCollector::default(),
		}
	}
}

impl UpdatableWidget for DiskWidget<'_> {
	fn update(&mut self) {
		let io_counters = self.collector.disk_io_counters_perdisk().unwrap();
		self.partitions = disk::partitions_physical()
			.unwrap()
			.into_iter()
			.map(|partition| {
				let mut name = PathBuf::from(partition.device())
					.file_name()
					.unwrap()
					.to_string_lossy()
					.to_string();
				// TODO: just going with it for now
				if name == "cryptroot" {
					name = "dm-0".to_string();
				}
				let mountpoint = partition.mountpoint().to_path_buf();

				let disk_usage = disk::disk_usage(&mountpoint).unwrap();

				let bytes_read = io_counters[&name].read_count();
				let bytes_written = io_counters[&name].read_count();
				let (bytes_read_recently, bytes_written_recently) = self
					.partitions
					.get(&name)
					.map(|other| {
						(
							bytes_read - other.bytes_read,
							bytes_written - other.bytes_written,
						)
					})
					.unwrap_or_default();
				let used_percent = disk_usage.percent();
				let bytes_free = disk_usage.free();

				(
					name.clone(),
					Partition {
						name,
						mountpoint,
						bytes_read,
						bytes_written,
						bytes_read_recently,
						bytes_written_recently,
						used_percent,
						bytes_free,
					},
				)
			})
			.collect();
	}

	fn get_update_interval(&self) -> Ratio<u64> {
		self.update_interval
	}
}

impl Widget for DiskWidget<'_> {
	fn draw(&mut self, area: Rect, buf: &mut Buffer) {
		let mut partitions: Vec<Partition> = self
			.partitions
			.iter()
			.map(|(_key, val)| val.clone())
			.collect();
		partitions.sort_by(|a, b| a.name.cmp(&b.name));

		Table::new(
			["Disk", "Mount", "Used", "Free", "R/s", "W/s"].iter(),
			partitions.iter().map(|partition| {
				Row::StyledData(
					vec![
						partition.name.to_string(),
						format!("{}", partition.mountpoint.display()),
						format!("{:3.0}%", partition.used_percent),
					]
					.into_iter(),
					self.colorscheme.text,
				)
			}),
		)
		.block(block::new(self.colorscheme, &self.title))
		.header_style(self.colorscheme.text.modifier(Modifier::BOLD))
		.widths(&[
			Constraint::Length(20),
			Constraint::Length(20),
			Constraint::Length(10),
			Constraint::Length(10),
			Constraint::Length(10),
			Constraint::Length(10),
		])
		.column_spacing(1)
		.draw(area, buf);
	}
}
