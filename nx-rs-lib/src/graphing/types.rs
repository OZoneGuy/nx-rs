#![allow(dead_code)]
use std::collections::{HashMap, HashSet};

use pathfinding::prelude::topological_sort;

type TaskID = String;

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    id: TaskID,
    name: String,
    action: Action,
}

/// Actions define different actions that a task can do.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Run a shell command
    Shell(Vec<String>),
}

impl Action {
    fn run(&self) {
        match self {
            Action::Shell(cmd) => {
                // TODO: add support for environment variables
                let mut child = std::process::Command::new(&cmd[0])
                    .args(&cmd[1..])
                    .spawn()
                    .expect("failed to execute process");
                child.wait().expect("failed to wait on child");
            }
        }
    }
}

/// A task graph is a directed acyclic graph (DAG) where each node is a task
/// and each edge is a dependency.
/// a -> b means that a depends on b. So b must be done before a.
/// The nodes are stored in a hashmap for easy access, TaskID -> Task.
/// The edges are stored in a hashmap for easy access, TaskID -> Vec<TaskID>.
#[derive(Debug)]
pub struct TaskGraph {
    /// The tasks in the graph. Use the ID to get the task
    tasks: HashMap<TaskID, Task>,
    /// Edges are stored as a -> (b, c, d).
    /// This means a depends on b, c, and d. So tasks b, c, and d must be done
    /// before starting with a
    edges: HashMap<TaskID, Vec<TaskID>>,
    /// Tasks that are done.
    done: HashSet<TaskID>,
    /// Tasks ordered using topological sort
    ordered_tasks: Vec<TaskID>,
}

impl TaskGraph {
    pub fn done(&mut self, task_id: &TaskID) {
        self.done.insert(task_id.clone());
    }

    pub fn remaining(&self) -> usize {
        self.ordered_tasks.len()
    }
}

impl Iterator for TaskGraph {
    // Some means a task is ready to be run
    // None means there are tasks remaining but none are ready to be run
    type Item = Option<Task>;

    // Has to be called manually due to returning None even when there are tasks
    // left
    fn next(&mut self) -> Option<Self::Item> {
        // task candidate
        // Later tasks should not be possible before this task

        // loop through all the tasks to find a task that is ready to be run
        if self.ordered_tasks.len() == 0 {
            return None;
        }

        for i in 0..self.ordered_tasks.len() {
            let task_id = self.ordered_tasks.get(i)?.clone();

            // If all dependencies are done
            if self.edges.get(&task_id).is_none()
                || self.done.is_superset(
                    &self
                        .edges
                        .get(&task_id)
                        .unwrap()
                        .iter()
                        .cloned()
                        .collect::<HashSet<TaskID>>(),
                )
            {
                // return the task
                self.ordered_tasks.remove(i);
                return Some(Some(self.tasks.get(&task_id).unwrap().clone()));
            }
        }
        return Some(None);
    }
}

#[derive(Clone)]
pub struct TaskGraphBuilder {
    tasks: HashMap<TaskID, Task>,
    edges: HashMap<TaskID, Vec<TaskID>>,
}

impl TaskGraphBuilder {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            edges: HashMap::new(),
        }
    }

    pub fn add_task(&mut self, task: Task) {
        self.edges.entry(task.id.clone()).or_insert_with(Vec::new);
        self.tasks.insert(task.id.clone(), task);
    }

    /// Add a dependency to a task
    /// # Arguments
    /// * `task` - The task to add a dependency to
    /// * `dependency` - The task that the task depends on
    pub fn add_dependency(&mut self, task: TaskID, dependency: TaskID) {
        self.edges
            .entry(task)
            .or_insert_with(Vec::new)
            .push(dependency);
    }

    /// Build the task graph
    /// Also creates the ordered tasks
    /// # Returns
    /// * `TaskGraph` - The task graph
    pub fn build(self) -> Result<TaskGraph, TaskID> {
        let edges = self.edges.clone();
        let start_edges = edges
            .iter()
            .filter(|(_, ts)| ts.len() == 0)
            .map(|(t, _)| t.clone())
            .clone()
            .collect::<Vec<TaskID>>();
        let successors = |task_id: &TaskID| -> Vec<String> {
            // Edges stored as a -> (b, c, d). Filter get all `a` where a -> (..., task_id, ...)
            // Since a -> b means a depends on b. Getting the `b` first means we
            // are getting all the dependencies first
            edges
                .iter()
                .filter(|(_, ts)| ts.contains(task_id))
                .map(|(t, _)| t.clone())
                .clone()
                .collect::<Vec<TaskID>>()
        };

        let ordered_tasks = topological_sort(&start_edges, successors)?;

        return Ok(TaskGraph {
            tasks: self.tasks,
            edges: self.edges,
            done: HashSet::new(),
            ordered_tasks,
        });
    }
}

#[cfg(test)]
mod test {

    use super::*;

    fn task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            name: id.to_string(),
            action: Action::Shell(vec![]),
        }
    }

    #[test]
    fn test_graph_creation() {
        let n_a = task("a");
        let n_b = task("b");
        let n_c = task("c");
        let n_d = task("d");

        let mut builder = TaskGraphBuilder::new();

        builder.add_task(n_a.clone());
        builder.add_task(n_b.clone());
        builder.add_task(n_c.clone());
        builder.add_task(n_d.clone());

        builder.add_dependency(n_a.id.clone(), n_b.id.clone());
        builder.add_dependency(n_a.id.clone(), n_d.id.clone());
        builder.add_dependency(n_b.id.clone(), n_c.id.clone());

        assert!(builder.clone().build().is_ok(), "Graph should be valid");

        builder.add_dependency(n_d.id.clone(), n_c.id.clone());
        assert!(
            builder.clone().build().is_ok(),
            "Graph should be valid with shared dependencies"
        );

        builder.add_dependency(n_d.id.clone(), n_b.id.clone());
        assert!(
            builder.clone().build().is_ok(),
            "Graph should be valid with multiple shared dependencies"
        );

        builder.add_dependency(n_b.id.clone(), n_d.id.clone());
        assert!(
            builder.clone().build().is_err(),
            "Graph should be invalid with circular dependencies"
        );
    }

    #[test]
    fn test_get_tasks() {
        for _ in 0..10 {
            let n_a = task("a");
            let n_b = task("b");
            let n_c = task("c");
            let n_d = task("d");

            let mut builder = TaskGraphBuilder::new();

            builder.add_task(n_a.clone());
            builder.add_task(n_b.clone());
            builder.add_task(n_c.clone());
            builder.add_task(n_d.clone());

            builder.add_dependency(n_a.id.clone(), n_b.id.clone());
            builder.add_dependency(n_a.id.clone(), n_d.id.clone());
            builder.add_dependency(n_a.id.clone(), n_c.id.clone());
            builder.add_dependency(n_b.id.clone(), n_c.id.clone());

            let mut graph = builder.build().unwrap();

            let first = graph.next().unwrap().unwrap();
            assert!(
                vec![n_c.clone(), n_d.clone()].contains(&first),
                "First task should be c or d",
            );

            if first == n_d {
                // first task is d, so we need to do c first
                graph.done(&n_d.id);
                assert_eq!(
                    graph.next(),
                    Some(Some(n_c.clone())),
                    "Next task should be c"
                );
            } else {
                // else first task is c, so we need to do d first
                assert_eq!(
                    graph.next(),
                    Some(Some(n_d.clone())),
                    "Next task should be d"
                );

                graph.done(&n_d.id);
            }

            assert_eq!(
                graph.next(),
                Some(None),
                "There are tasks remaining, but none are ready"
            );

            graph.done(&n_c.id);
            assert_eq!(
                graph.next(),
                Some(Some(n_b.clone())),
                "Next task should be b"
            );
            graph.done(&n_b.id);

            assert_eq!(
                graph.next(),
                Some(Some(n_a.clone())),
                "Final task should be a"
            );
            graph.done(&n_a.id);

            assert_eq!(
                graph.next(),
                None,
                "Should return None if no tasks are ready"
            );

            // remaining tasks should be 0
            assert_eq!(graph.remaining(), 0, "Should have no remaining tasks");
        }
    }
}
