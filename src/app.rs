use std::collections::BTreeSet;

use ratatui::widgets::ListState;

use crate::config::{Config, SkillState};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Skills,
    Groups,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupStatus {
    Active,
    Inactive,
    Mixed,
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteTarget {
    Skill(String),
    Group(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupNameMode {
    Create,
    Rename { original: String },
}

#[derive(Debug, Clone)]
pub struct GroupNameDialog {
    pub mode: GroupNameMode,
    pub value: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GroupEditorEntry {
    pub key: String,
    pub name: String,
    pub active: bool,
    pub missing: bool,
}

#[derive(Debug, Clone)]
pub struct GroupEditor {
    pub group_name: String,
    pub entries: Vec<GroupEditorEntry>,
    pub members: BTreeSet<String>,
    pub selected: usize,
    pub list_state: ListState,
}

pub struct App {
    pub config: Config,
    pub skills: Vec<Skill>,
    pub selected: usize,
    pub list_state: ListState,
    pub group_selected: usize,
    pub group_list_state: ListState,
    pub search_query: String,
    pub searching: bool,
    pub running: bool,
    pub screen: Screen,
    pub focus: Focus,

    // Import state
    pub unmanaged: Vec<UnmanagedSkill>,
    pub import_confirm: ImportConfirm,

    // Main screen overlays
    pub delete_confirm: Option<DeleteTarget>,
    pub group_name_input: Option<GroupNameDialog>,
    pub group_editor: Option<GroupEditor>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let skills = skills::load_managed_skills(&config);
        let list_state = ListState::default();
        let group_list_state = ListState::default();
        let mut app = Self {
            config,
            skills,
            selected: 0,
            list_state,
            group_selected: 0,
            group_list_state,
            search_query: String::new(),
            searching: false,
            running: true,
            screen: Screen::Main,
            focus: Focus::Skills,
            unmanaged: Vec::new(),
            import_confirm: ImportConfirm::Yes,
            delete_confirm: None,
            group_name_input: None,
            group_editor: None,
        };
        app.sync_skill_selection();
        app.sync_group_selection();
        app
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
        self.sync_skill_selection();
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

    pub fn group_entries(&self) -> Vec<(&String, &Vec<String>)> {
        self.config.groups.iter().collect()
    }

    pub fn selected_group(&self) -> Option<(&String, &Vec<String>)> {
        let groups = self.group_entries();
        groups.get(self.group_selected).copied()
    }

    pub fn groups_for_skill<'a>(&'a self, key: &str) -> Vec<&'a str> {
        self.config
            .groups
            .iter()
            .filter(|(_, members)| members.iter().any(|member| member == key))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    pub fn group_member_skills<'a>(&'a self, group_name: &str) -> Vec<&'a Skill> {
        let Some(members) = self.config.groups.get(group_name) else {
            return Vec::new();
        };

        members
            .iter()
            .filter_map(|member| self.skills.iter().find(|skill| skill.key == *member))
            .collect()
    }

    pub fn group_status(&self, group_name: &str) -> GroupStatus {
        let members = self.group_member_skills(group_name);
        let managed_count = members.len();
        let active_count = members.iter().filter(|skill| skill.active).count();

        match (managed_count, active_count) {
            (0, _) => GroupStatus::Empty,
            (_, 0) => GroupStatus::Inactive,
            (managed, active) if managed == active => GroupStatus::Active,
            _ => GroupStatus::Mixed,
        }
    }

    pub fn group_counts(&self, group_name: &str) -> (usize, usize, usize) {
        let configured_count = self
            .config
            .groups
            .get(group_name)
            .map(|members| members.len())
            .unwrap_or(0);
        let members = self.group_member_skills(group_name);
        let managed_count = members.len();
        let active_count = members.iter().filter(|skill| skill.active).count();
        (active_count, managed_count, configured_count)
    }

    pub fn move_skill_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.list_state.select(Some(self.selected));
        }
    }

    pub fn move_skill_down(&mut self) {
        let max = self.filtered_skills().len().saturating_sub(1);
        if self.selected < max {
            self.selected = max.min(self.selected + 1);
            self.list_state.select(Some(self.selected));
        }
    }

    pub fn move_group_up(&mut self) {
        if self.group_selected > 0 {
            self.group_selected -= 1;
            self.group_list_state.select(Some(self.group_selected));
        }
    }

    pub fn move_group_down(&mut self) {
        let max = self.group_entries().len().saturating_sub(1);
        if self.group_selected < max {
            self.group_selected = max.min(self.group_selected + 1);
            self.group_list_state.select(Some(self.group_selected));
        }
    }

    pub fn toggle_selected_skill(&mut self) {
        let filtered = self.filtered_skills();
        if let Some(&(real_idx, skill)) = filtered.get(self.selected) {
            let key = skill.key.clone();
            let next_active = !self.skills[real_idx].active;
            if self.set_skill_states(&[key], next_active) {
                self.persist_skill_state();
            }
        }
    }

    pub fn toggle_selected_group(&mut self) {
        let Some((group_name, members)) = self.selected_group() else {
            return;
        };

        let keys = members.clone();
        let next_active = self.group_status(group_name) != GroupStatus::Active;
        if self.set_skill_states(&keys, next_active) {
            self.persist_skill_state();
        }
    }

    pub fn activate_all(&mut self) {
        let keys = self
            .skills
            .iter()
            .map(|skill| skill.key.clone())
            .collect::<Vec<_>>();
        if self.set_skill_states(&keys, true) {
            self.persist_skill_state();
        }
    }

    pub fn deactivate_all(&mut self) {
        let keys = self
            .skills
            .iter()
            .map(|skill| skill.key.clone())
            .collect::<Vec<_>>();
        if self.set_skill_states(&keys, false) {
            self.persist_skill_state();
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Skills => Focus::Groups,
            Focus::Groups => Focus::Skills,
        };
    }

    pub fn focus_skills(&mut self) {
        self.focus = Focus::Skills;
    }

    pub fn request_new_group(&mut self) {
        self.group_name_input = Some(GroupNameDialog {
            mode: GroupNameMode::Create,
            value: String::new(),
            error: None,
        });
        self.focus = Focus::Groups;
    }

    pub fn request_rename_group(&mut self) {
        let Some((group_name, _)) = self.selected_group() else {
            return;
        };

        self.group_name_input = Some(GroupNameDialog {
            mode: GroupNameMode::Rename {
                original: group_name.clone(),
            },
            value: group_name.clone(),
            error: None,
        });
        self.focus = Focus::Groups;
    }

    pub fn cancel_group_name_input(&mut self) {
        self.group_name_input = None;
    }

    pub fn group_name_push(&mut self, c: char) {
        if let Some(dialog) = &mut self.group_name_input {
            dialog.value.push(c);
            dialog.error = None;
        }
    }

    pub fn group_name_pop(&mut self) {
        if let Some(dialog) = &mut self.group_name_input {
            dialog.value.pop();
            dialog.error = None;
        }
    }

    pub fn submit_group_name_input(&mut self) {
        let Some(dialog) = &self.group_name_input else {
            return;
        };

        let mode = dialog.mode.clone();
        let name = dialog.value.trim().to_string();

        if name.is_empty() {
            self.set_group_name_error("Group name cannot be empty.");
            return;
        }

        match mode {
            GroupNameMode::Create => {
                if self.config.groups.contains_key(&name) {
                    self.set_group_name_error("A group with that name already exists.");
                    return;
                }

                self.config.groups.insert(name.clone(), Vec::new());
                self.group_name_input = None;
                self.persist_groups();
                self.select_group_by_name(&name);
                self.request_edit_group();
            }
            GroupNameMode::Rename { original } => {
                if name != original && self.config.groups.contains_key(&name) {
                    self.set_group_name_error("A group with that name already exists.");
                    return;
                }

                let Some(members) = self.config.groups.remove(&original) else {
                    self.group_name_input = None;
                    self.sync_group_selection();
                    return;
                };

                self.config.groups.insert(name.clone(), members);
                self.group_name_input = None;
                self.persist_groups();
                self.select_group_by_name(&name);
            }
        }
    }

    pub fn request_edit_group(&mut self) {
        let Some((group_name, member_keys)) = self.selected_group() else {
            return;
        };

        let mut entries = self
            .skills
            .iter()
            .map(|skill| GroupEditorEntry {
                key: skill.key.clone(),
                name: skill.meta.name.clone(),
                active: skill.active,
                missing: false,
            })
            .collect::<Vec<_>>();

        for member_key in member_keys {
            if self.skills.iter().any(|skill| skill.key == *member_key) {
                continue;
            }

            entries.push(GroupEditorEntry {
                key: member_key.clone(),
                name: member_key.clone(),
                active: false,
                missing: true,
            });
        }

        let mut list_state = ListState::default();
        let selected = 0;
        if entries.is_empty() {
            list_state.select(None);
        } else {
            list_state.select(Some(selected));
        }

        self.group_editor = Some(GroupEditor {
            group_name: group_name.clone(),
            entries,
            members: member_keys.iter().cloned().collect(),
            selected,
            list_state,
        });
        self.focus = Focus::Groups;
    }

    pub fn cancel_group_editor(&mut self) {
        self.group_editor = None;
    }

    pub fn save_group_editor(&mut self) {
        let Some(editor) = self.group_editor.take() else {
            return;
        };

        let members = editor
            .entries
            .iter()
            .filter(|entry| editor.members.contains(&entry.key))
            .map(|entry| entry.key.clone())
            .collect::<Vec<_>>();

        if let Some(group_members) = self.config.groups.get_mut(&editor.group_name) {
            *group_members = members;
            self.persist_groups();
            self.select_group_by_name(&editor.group_name);
        } else {
            self.sync_group_selection();
        }
    }

    pub fn toggle_group_editor_member(&mut self) {
        let Some(editor) = &mut self.group_editor else {
            return;
        };

        let Some(entry) = editor.entries.get(editor.selected) else {
            return;
        };

        if !editor.members.insert(entry.key.clone()) {
            editor.members.remove(&entry.key);
        }
    }

    pub fn move_group_editor_up(&mut self) {
        let Some(editor) = &mut self.group_editor else {
            return;
        };

        if editor.selected > 0 {
            editor.selected -= 1;
            editor.list_state.select(Some(editor.selected));
        }
    }

    pub fn move_group_editor_down(&mut self) {
        let Some(editor) = &mut self.group_editor else {
            return;
        };

        let max = editor.entries.len().saturating_sub(1);
        if editor.selected < max {
            editor.selected = max.min(editor.selected + 1);
            editor.list_state.select(Some(editor.selected));
        }
    }

    pub fn request_delete_skill(&mut self) {
        if let Some(skill) = self.selected_skill() {
            self.delete_confirm = Some(DeleteTarget::Skill(skill.key.clone()));
        }
    }

    pub fn request_delete_group(&mut self) {
        if let Some((group_name, _)) = self.selected_group() {
            self.delete_confirm = Some(DeleteTarget::Group(group_name.clone()));
        }
    }

    fn set_skill_states(&mut self, keys: &[String], active: bool) -> bool {
        let mut changed = false;

        for key in keys {
            let Some(skill) = self.skills.iter_mut().find(|skill| skill.key == *key) else {
                continue;
            };

            if skill.active != active {
                skill.active = active;
                changed = true;
            }

            if self.config.skills.get(key).map(|state| state.active) != Some(active) {
                self.config
                    .skills
                    .insert(key.clone(), SkillState { active });
                changed = true;
            }
        }

        changed
    }

    fn persist_skill_state(&mut self) {
        self.sync_skill_selection();
        self.sync_group_selection();
        self.config.save();
        skills::sync_symlinks(&self.config);
    }

    fn persist_groups(&mut self) {
        self.sync_group_selection();
        self.config.save();
    }

    fn sync_skill_selection(&mut self) {
        let filtered_len = self.filtered_skills().len();
        if filtered_len == 0 {
            self.selected = 0;
            self.list_state.select(None);
        } else {
            if self.selected >= filtered_len {
                self.selected = filtered_len - 1;
            }
            self.list_state.select(Some(self.selected));
        }
    }

    fn select_group_by_name(&mut self, name: &str) {
        if let Some(index) = self
            .config
            .groups
            .keys()
            .position(|group_name| group_name == name)
        {
            self.group_selected = index;
        }
        self.sync_group_selection();
    }

    fn set_group_name_error(&mut self, message: &str) {
        if let Some(dialog) = &mut self.group_name_input {
            dialog.error = Some(message.to_string());
        }
    }

    fn sync_group_selection(&mut self) {
        let group_len = self.config.groups.len();
        if group_len == 0 {
            self.group_selected = 0;
            self.group_list_state.select(None);
            self.focus = Focus::Skills;
        } else {
            if self.group_selected >= group_len {
                self.group_selected = group_len - 1;
            }
            self.group_list_state.select(Some(self.group_selected));
        }
    }

    pub fn start_search(&mut self) {
        self.searching = true;
        self.focus = Focus::Skills;
        self.search_query.clear();
        self.selected = 0;
        self.sync_skill_selection();
    }

    pub fn end_search(&mut self) {
        self.searching = false;
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.selected = 0;
        self.sync_skill_selection();
    }

    pub fn search_push(&mut self, c: char) {
        self.search_query.push(c);
        self.selected = 0;
        self.sync_skill_selection();
    }

    pub fn search_pop(&mut self) {
        self.search_query.pop();
        self.selected = 0;
        self.sync_skill_selection();
    }

    pub fn confirm_delete(&mut self) {
        if let Some(target) = self.delete_confirm.take() {
            match target {
                DeleteTarget::Skill(name) => {
                    skills::delete_skill(&name, &mut self.config);
                    self.skills = skills::load_managed_skills(&self.config);
                    self.sync_skill_selection();
                    self.sync_group_selection();
                }
                DeleteTarget::Group(name) => {
                    self.config.groups.remove(&name);
                    self.persist_groups();
                }
            }
        }
    }

    pub fn cancel_delete(&mut self) {
        self.delete_confirm = None;
    }
}
