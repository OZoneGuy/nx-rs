use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::read_to_string, path::Path};

/// The list of possible errors that can occur when validating the projects in
/// the workspace.
#[derive(Debug)]
pub enum ValidateProjectsError {
    /// The project is missing one or more of the required targets
    /// # Arguments
    /// * `String` - The name of the project
    /// * `String` - The name of the missing target
    MissingTargets(String, String),

    /// The project has one or more unknown tags
    /// # Arguments
    /// * `String` - The name of the project
    /// * `Vec<String>` - The list of unknown tags
    UnknownTags(String, Vec<String>),

    /// The project could not be deserialized
    /// # Arguments
    /// * `String` - The name of the project
    ProjectSerialization(String),

    /// The workspace could not be deserialized
    WorkspaceSerialization,
}

// NOTE: should I use the same one from the algorithms module, or create a new
// one?
#[derive(Serialize, Deserialize, Debug)]
pub struct Target {}

#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    name: String,
    version: Option<String>,
    description: String,
    owners: Vec<String>,
    affects_tags: Vec<String>,
    affected_by_tags: Vec<String>,
    targets: HashMap<String, Target>,
}

impl Project {
    pub fn read(path: &Path) -> Result<Project> {
        let data = read_to_string(path)?;

        let proj = serde_json::from_str(&data)?;
        return Ok(proj);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Workspace {
    name: String,
    app_version: String,
    projects: HashMap<String, String>,
    tags: Vec<String>,
    maintainers: Vec<String>,
    repository: String,
    required_targets: Vec<String>,
}

impl Workspace {
    pub fn read(path: &Path) -> Result<Workspace> {
        let data = read_to_string(path)?;

        let ws = serde_json::from_str(&data)?;
        return Ok(ws);
    }

    /// Returns the list of projects that are affected by the given project
    /// based on the tags set on the project.
    /// # Arguments
    /// * `proj_name` - The name of the project to check
    ///
    /// # Returns
    /// * `Vec<String>` - The list of projects affected by the given project
    pub fn affected_projects(&self, proj_name: &str) -> Result<Vec<String>> {
        return Ok(Workspace::affected_util(
            proj_name,
            &self.get_projects_map()?,
        ));
    }

    /// Returns the list of projects that are affected by the given project
    /// based on the tags set on the project. Does so recursively.
    /// # Arguments
    /// * `proj_name` - The name of the project to check
    /// * `projects` - A hashmap of all the projects in the workspace
    ///
    /// # Returns
    /// * `Vec<String>` - The list of projects affected by the given project
    fn affected_util(proj_name: &str, projects: &HashMap<String, Project>) -> Vec<String> {
        let mut affected: Vec<String> = vec![];

        let tags = projects.get(proj_name).unwrap().affects_tags.clone();

        for (name, proj) in projects {
            if proj.affected_by_tags.iter().any(|t| tags.contains(t)) {
                affected.push(name.clone());
                affected.extend(Workspace::affected_util(name, projects));
            }
        }

        return affected;
    }

    fn get_projects_map(&self) -> Result<HashMap<String, Project>> {
        let mut projects: HashMap<String, Project> = HashMap::new();

        for (name, path) in &self.projects {
            let proj = Project::read(Path::new(path))?;
            projects.insert(name.clone(), proj);
        }

        return Ok(projects);
    }

    /// Returns a list of validation errors for the workspace.
    /// See `ValidateProjectsError` for the list of possible errors.
    /// # Returns
    /// * `Vec<ValidateProjectsError>` - The list of validation errors
    pub fn validate_projects() -> Vec<ValidateProjectsError> {
        let ws_res = Workspace::read(Path::new("workspace.json"));

        let ws: Workspace;
        if let Ok(w) = ws_res {
            ws = w;
        } else {
            return vec![ValidateProjectsError::WorkspaceSerialization];
        }

        let mut errors: Vec<ValidateProjectsError> = vec![];

        for (name, path) in ws.projects {
            let proj_res = Project::read(Path::new(&path));

            let proj: Project;
            if let Ok(p) = proj_res {
                proj = p;
            } else {
                errors.push(ValidateProjectsError::ProjectSerialization(name.clone()));
                continue;
            }

            // check targets
            for target in ws.required_targets.clone() {
                if !proj.targets.contains_key(&target) {
                    errors.push(ValidateProjectsError::MissingTargets(name.clone(), target));
                }
            }

            // check tags
            let mut unknown_tags: Vec<String> = vec![];
            for tag in proj.affects_tags.clone() {
                if !ws.tags.contains(&tag) {
                    unknown_tags.push(tag);
                }
            }

            for tag in proj.affected_by_tags.clone() {
                if !ws.tags.contains(&tag) {
                    unknown_tags.push(tag);
                }
            }

            if unknown_tags.is_empty() {
                errors.push(ValidateProjectsError::UnknownTags(
                    name.clone(),
                    unknown_tags,
                ));
            }
        }

        return errors;
    }
}
