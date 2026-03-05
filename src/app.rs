use ratatui::widgets::ListState;

use crate::config::Config;
use crate::skills::{self, Skill, UnmanagedSkill};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen {
    Import,
    Main,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImportConfirm {
    Yes,
    No,
}

pub struct App {
    pub config: Config,
    pub skills: Vec<Skill>,
    pub selected: usize,
    pub list_state: ListState,
    pub search_query: String,
    pub searching: bool,
    pub running: bool,
    pub screen: Screen,

    // Import state
    pub unmanaged: Vec<UnmanagedSkill>,
    pub import_confirm: ImportConfirm,

    // Delete confirmation
    pub delete_confirm: Option<String>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let skills = skills::load_managed_skills(&config);
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            config,
            skills,
            selected: 0,
            list_state,
            search_query: String::new(),
            searching: false,
            running: true,
            screen: Screen::Main,
            unmanaged: Vec::new(),
            import_confirm: ImportConfirm::Yes,
            delete_confirm: None,
        }
    }

    pub fn check_unmanaged(&mut self) {
        self.unmanaged = skills::find_unmanaged_skills(&self.config);
        if !self.unmanaged.is_empty() {
            self.screen = Screen::Import;
        }
    }

    pub fn confirm_import(&mut self) {
        skills::import_skills(&self.unmanaged, &mut self.config);
        self.config.save();
        self.unmanaged.clear();
        self.skills = skills::load_managed_skills(&self.config);
        self.screen = Screen::Main;
    }

    pub fn skip_import(&mut self) {
        self.unmanaged.clear();
        self.screen = Screen::Main;
    }

    pub fn filtered_skills(&self) -> Vec<(usize, &Skill)> {
        if self.search_query.is_empty() {
            self.skills.iter().enumerate().collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.skills
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.meta.name.to_lowercase().contains(&q)
                        || s.meta.description.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    pub fn selected_skill(&self) -> Option<&Skill> {
        let filtered = self.filtered_skills();
        filtered.get(self.selected).map(|(_, s)| *s)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.list_state.select(Some(self.selected));
        }
    }

    pub fn move_down(&mut self) {
        let max = self.filtered_skills().len().saturating_sub(1);
        if self.selected < max {
            self.selected = max.min(self.selected + 1);
            self.list_state.select(Some(self.selected));
        }
    }

    pub fn toggle_selected(&mut self) {
        let filtered = self.filtered_skills();
        if let Some(&(real_idx, _)) = filtered.get(self.selected) {
            self.skills[real_idx].active = !self.skills[real_idx].active;

            let name = self.skills[real_idx].meta.name.clone();
            let active = self.skills[real_idx].active;

            self.config.skills.insert(
                name,
                crate::config::SkillState { active },
            );
            self.config.save();
            skills::sync_symlinks(&self.config);
        }
    }

    pub fn start_search(&mut self) {
        self.searching = true;
        self.search_query.clear();
        self.selected = 0;
        self.list_state.select(Some(0));
    }

    pub fn end_search(&mut self) {
        self.searching = false;
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.selected = 0;
        self.list_state.select(Some(0));
    }

    pub fn search_push(&mut self, c: char) {
        self.search_query.push(c);
        self.selected = 0;
        self.list_state.select(Some(0));
    }

    pub fn search_pop(&mut self) {
        self.search_query.pop();
        self.selected = 0;
        self.list_state.select(Some(0));
    }

    pub fn request_delete(&mut self) {
        if let Some(skill) = self.selected_skill() {
            self.delete_confirm = Some(skill.meta.name.clone());
        }
    }

    pub fn confirm_delete(&mut self) {
        if let Some(name) = self.delete_confirm.take() {
            skills::delete_skill(&name, &mut self.config);
            self.skills = skills::load_managed_skills(&self.config);
            let max = self.skills.len().saturating_sub(1);
            if self.selected > max {
                self.selected = max;
            }
            self.list_state.select(Some(self.selected));
        }
    }

    pub fn cancel_delete(&mut self) {
        self.delete_confirm = None;
    }
}
