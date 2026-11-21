//! Starter sections shared by the template center and empty documents.

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Template {
    pub(crate) title: &'static str,
    pub(crate) sections: &'static [&'static str],
}

impl Template {
    pub(crate) fn markdown(self) -> String {
        use std::fmt::Write as _;
        let mut markdown = String::new();
        for heading in self.sections {
            let _ = writeln!(markdown, "## {heading}\n");
        }
        markdown
    }
}

pub(crate) const TEMPLATES: [Template; 3] = [
    Template {
        title: "走查记录",
        sections: &["走查范围", "观察记录", "下一步"],
    },
    Template {
        title: "会议纪要",
        sections: &["议题", "决议", "下一步行动"],
    },
    Template {
        title: "项目周记",
        sections: &["本周进展", "待解决的问题", "下周计划"],
    },
];
