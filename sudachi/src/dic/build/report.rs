/*
 *  Copyright (c) 2021 Works Applications Co., Ltd.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use std::time::{Duration, Instant};

pub struct DictPartReport {
    pub(self) part: String,
    pub(self) time: Duration,
    pub(self) size: usize,
    pub(self) write: bool,
}

impl DictPartReport {
    pub fn part(&self) -> &str {
        &self.part
    }

    pub fn time(&self) -> Duration {
        self.time
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn is_write(&self) -> bool {
        self.write
    }
}

pub(crate) struct Reporter {
    reports: Vec<DictPartReport>,
}

impl Reporter {
    pub fn new() -> Reporter {
        Self {
            reports: Vec::with_capacity(10),
        }
    }

    pub fn collect(&mut self, end: usize, report: ReportBuilder) {
        let mut rep = report.report;
        rep.time = Instant::now().duration_since(report.start);
        rep.size = end;
        self.reports.push(rep);
    }

    pub fn collect_r<T>(
        &mut self,
        end: Result<usize, T>,
        report: ReportBuilder,
    ) -> Result<usize, T> {
        match end {
            Ok(s) => {
                self.collect(s, report);
                Ok(s)
            }
            e => e,
        }
    }

    pub fn reports(&self) -> &[DictPartReport] {
        &self.reports
    }
}

pub(crate) struct ReportBuilder {
    start: Instant,
    report: DictPartReport,
}

impl ReportBuilder {
    pub fn new<S: Into<String>>(desc: S) -> Self {
        Self {
            start: Instant::now(),
            report: DictPartReport {
                part: desc.into(),
                size: 0,
                time: Duration::default(),
                write: true,
            },
        }
    }

    pub fn read(mut self) -> Self {
        self.report.write = false;
        self
    }
}
