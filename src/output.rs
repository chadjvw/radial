use std::io::{self, Write};

use anyhow::Result;
use console::{style, Term};
use serde::Serialize;
use textwrap::wrap;

use crate::commands::status::{GoalStatus, GoalSummary, StatusResult};
use crate::commands::task::CompleteResult;
use crate::models::{Goal, Task};

/// Trait for types that can render themselves as human-readable CLI output.
pub trait Render {
    fn render(&self, w: &mut dyn Write) -> Result<()>;
}

/// Print as JSON if `json` is true, otherwise call `human` with a writer.
fn json_or<T: Serialize + ?Sized>(
    value: &T,
    json: bool,
    human: impl FnOnce(&mut dyn Write) -> Result<()>,
) -> Result<()> {
    let mut stdout = io::stdout().lock();
    if json {
        serde_json::to_writer_pretty(&mut stdout, value)?;
        writeln!(stdout)?;
    } else {
        human(&mut stdout)?;
    }
    Ok(())
}

fn terminal_width() -> usize {
    let (_, cols) = Term::stdout().size();
    cols as usize
}

/// Write a labeled field, wrapping long or multiline values.
///
/// Short values print inline: `{prefix}{label}: {value}`
/// Long or multiline values wrap onto indented continuation lines:
/// ```text
/// {prefix}{label}:
/// {prefix}  {wrapped line 1}
/// {prefix}  {wrapped line 2}
/// ```
pub fn write_field(w: &mut dyn Write, prefix: &str, label: &str, value: &str) -> Result<()> {
    let width = terminal_width();
    let inline_prefix = format!("{prefix}{label}: ");
    let inline_len = inline_prefix.len() + value.len();

    if !value.contains('\n') && inline_len <= width {
        writeln!(w, "{inline_prefix}{value}")?;
    } else {
        writeln!(w, "{prefix}{label}:")?;
        let continuation = format!("{prefix}  ");
        let wrap_width = width.saturating_sub(continuation.len()).max(20);
        for paragraph in value.split('\n') {
            if paragraph.is_empty() {
                writeln!(w)?;
            } else {
                for line in wrap(paragraph, wrap_width) {
                    writeln!(w, "{continuation}{line}")?;
                }
            }
        }
    }
    Ok(())
}

// -- Goal outputs --

pub fn goal_created(goal: &Goal, json: bool) -> Result<()> {
    json_or(goal, json, |w| {
        writeln!(
            w,
            "{} {}",
            style("Created goal:").green(),
            style(&goal.id).cyan().bold()
        )?;
        write_field(w, "  ", "Description", &goal.description)?;
        Ok(())
    })
}

pub fn goal_list(goals: &[Goal], json: bool) -> Result<()> {
    json_or(goals, json, |w| {
        if goals.is_empty() {
            writeln!(w, "No goals found.")?;
            return Ok(());
        }

        for goal in goals {
            goal.render(w)?;
            writeln!(w)?;
        }
        Ok(())
    })
}

// -- Task outputs --

pub fn task_created(task: &Task, json: bool) -> Result<()> {
    json_or(task, json, |w| {
        writeln!(
            w,
            "{} {}",
            style("Created task:").green(),
            style(&task.id).cyan().bold()
        )?;
        write_field(w, "  ", "Description", &task.description)?;
        writeln!(w, "  State: {}", style(task.state.as_ref()).yellow())?;
        if task.contract.is_none() {
            writeln!(
                w,
                "  Contract: {}",
                style("(not set - required before starting)").dim()
            )?;
        }
        Ok(())
    })
}

pub fn task_list(tasks: &[Task], goal: &Goal, verbose: bool, json: bool) -> Result<()> {
    json_or(tasks, json, |w| {
        writeln!(
            w,
            "Tasks for goal: {} [{}]",
            style(&goal.id).cyan().bold(),
            style(goal.state.as_ref()).yellow()
        )?;
        write_field(w, "  ", "Description", &goal.description)?;
        writeln!(w)?;

        if tasks.is_empty() {
            writeln!(w, "No tasks found.")?;
            return Ok(());
        }

        for task in tasks {
            task.render(w)?;
            if verbose && !task.comments.is_empty() {
                writeln!(w, "  Comments: ({})", task.comments.len())?;
                for comment in &task.comments {
                    writeln!(w, "    [{}]", style(&comment.created_at).dim())?;
                    write_field(w, "    ", "", &comment.text)?;
                }
            }
            writeln!(w)?;
        }
        Ok(())
    })
}

pub fn task_started(task: &Task) -> Result<()> {
    let mut w = io::stdout().lock();
    writeln!(
        w,
        "{} {}",
        style("Started task:").green(),
        style(&task.id).cyan().bold()
    )?;
    write_field(&mut w, "  ", "Description", &task.description)?;
    Ok(())
}

pub fn task_completed(result: &CompleteResult) -> Result<()> {
    let mut w = io::stdout().lock();
    writeln!(
        w,
        "{} {}",
        style("Completed task:").green(),
        style(&result.task.id).cyan().bold()
    )?;
    if let Some(ref res) = result.task.result {
        write_field(&mut w, "  ", "Result", &res.summary)?;
    }

    if !result.unblocked_task_ids.is_empty() {
        writeln!(w)?;
        writeln!(w, "{}", style("Unblocked tasks:").yellow())?;
        for id in &result.unblocked_task_ids {
            writeln!(w, "  - {}", style(id).cyan())?;
        }
    }
    Ok(())
}

pub fn task_failed(task: &Task) -> Result<()> {
    let mut w = io::stdout().lock();
    writeln!(
        w,
        "{} {}",
        style("Failed task:").red(),
        style(&task.id).cyan().bold()
    )?;
    write_field(&mut w, "  ", "Description", &task.description)?;
    Ok(())
}

pub fn task_retry(task: &Task) -> Result<()> {
    let mut w = io::stdout().lock();
    writeln!(
        w,
        "{} {}",
        style("Retrying task:").yellow(),
        style(&task.id).cyan().bold()
    )?;
    write_field(&mut w, "  ", "Description", &task.description)?;
    writeln!(w, "  Retry count: {}", task.metrics.retry_count)?;
    Ok(())
}

pub fn task_commented(task: &Task, json: bool) -> Result<()> {
    json_or(task, json, |w| {
        writeln!(
            w,
            "{} {}",
            style("Added comment to task:").green(),
            style(&task.id).cyan().bold()
        )?;
        if let Some(comment) = task.comments.last() {
            write_field(w, "  ", "Comment", &comment.text)?;
        }
        writeln!(w, "  Total comments: {}", task.comments.len())?;
        Ok(())
    })
}

// -- Status outputs --

pub fn status(result: &StatusResult, json: bool, concise: bool) -> Result<()> {
    match result {
        StatusResult::Task(task) => status_task(task, json, concise),
        StatusResult::Goal(goal_status) => status_goal(goal_status, json),
        StatusResult::AllGoals(summaries) => status_all_goals(summaries, json),
    }
}

fn status_task(task: &Task, json: bool, concise: bool) -> Result<()> {
    json_or(task, json, |w| {
        writeln!(
            w,
            "Task: {} [{}]",
            style(&task.id).cyan().bold(),
            style(task.state.as_ref()).yellow()
        )?;
        writeln!(w, "  Goal: {}", task.goal_id)?;
        write_field(w, "  ", "Description", &task.description)?;
        writeln!(w, "  Created: {}", task.created_at)?;
        writeln!(w, "  Updated: {}", task.updated_at)?;
        writeln!(w)?;

        match task.contract {
            Some(ref contract) => {
                writeln!(w, "{}", style("Contract:").bold())?;
                write_field(w, "  ", "Receives", &contract.receives)?;
                write_field(w, "  ", "Produces", &contract.produces)?;
                write_field(w, "  ", "Verify", &contract.verify)?;
            }
            None => {
                writeln!(w, "Contract: {}", style("(not set)").dim())?;
            }
        }

        if !task.blocked_by.is_empty() {
            writeln!(w)?;
            writeln!(w, "Blocked by: {}", task.blocked_by.join(", "))?;
        }

        if let Some(result) = &task.result {
            writeln!(w)?;
            writeln!(w, "{}", style("Result:").bold())?;
            write_field(w, "  ", "Summary", &result.summary)?;
            if !result.artifacts.is_empty() {
                writeln!(w, "  Artifacts:")?;
                for artifact in &result.artifacts {
                    write_field(w, "    ", "-", artifact)?;
                }
            }
        }

        writeln!(w)?;
        writeln!(w, "{}", style("Metrics:").bold())?;
        writeln!(w, "  Tokens: {}", task.metrics.tokens)?;
        writeln!(w, "  Elapsed: {}ms", task.metrics.elapsed_ms)?;
        writeln!(w, "  Retries: {}", task.metrics.retry_count)?;

        if !concise && !task.comments.is_empty() {
            writeln!(w)?;
            writeln!(w, "{}", style("Comments:").bold())?;
            for comment in &task.comments {
                writeln!(w, "  [{}]", style(&comment.created_at).dim())?;
                write_field(w, "  ", "", &comment.text)?;
            }
        }

        Ok(())
    })
}

fn status_goal(goal_status: &GoalStatus, json: bool) -> Result<()> {
    json_or(goal_status, json, |w| {
        let goal = &goal_status.goal;
        let metrics = &goal_status.metrics;

        writeln!(
            w,
            "Goal: {} [{}]",
            style(&goal.id).cyan().bold(),
            style(goal.state.as_ref()).yellow()
        )?;
        write_field(w, "  ", "Description", &goal.description)?;
        writeln!(w, "  Created: {}", goal.created_at)?;
        writeln!(w, "  Updated: {}", goal.updated_at)?;
        if let Some(completed_at) = &goal.completed_at {
            writeln!(w, "  Completed: {completed_at}")?;
        }

        writeln!(w)?;
        writeln!(w, "{}", style("Metrics:").bold())?;
        metrics.render(w)?;

        if !goal_status.tasks.is_empty() {
            writeln!(w)?;
            writeln!(w, "{}", style("Tasks:").bold())?;
            for task in &goal_status.tasks {
                writeln!(
                    w,
                    "  {} [{}] - {}",
                    style(&task.id).cyan(),
                    style(task.state.as_ref()).yellow(),
                    task.description
                )?;
            }
        }
        Ok(())
    })
}

fn status_all_goals(summaries: &[GoalSummary], json: bool) -> Result<()> {
    json_or(summaries, json, |w| {
        if summaries.is_empty() {
            writeln!(w, "No goals found.")?;
            return Ok(());
        }

        writeln!(w, "{}\n", style("All Goals:").bold())?;

        for summary in summaries {
            let goal = &summary.goal;
            let metrics = &summary.computed_metrics;

            writeln!(
                w,
                "{} [{}]",
                style(&goal.id).cyan().bold(),
                style(goal.state.as_ref()).yellow()
            )?;
            write_field(w, "  ", "Description", &goal.description)?;
            metrics.render(w)?;
            writeln!(w)?;
        }
        Ok(())
    })
}

// -- Ready --

pub fn ready_tasks(tasks: &[Task], goal: &Goal, json: bool) -> Result<()> {
    json_or(tasks, json, |w| {
        writeln!(
            w,
            "Ready tasks for goal: {} [{}]",
            style(&goal.id).cyan().bold(),
            style(goal.state.as_ref()).yellow()
        )?;
        write_field(w, "  ", "Description", &goal.description)?;
        writeln!(w)?;

        if tasks.is_empty() {
            writeln!(w, "No tasks ready to start.")?;
            return Ok(());
        }

        writeln!(w, "{} task(s) ready:\n", style(tasks.len()).green().bold())?;

        for task in tasks {
            writeln!(w, "{}", style(&task.id).cyan().bold())?;
            write_field(w, "  ", "Description", &task.description)?;
            if let Some(ref contract) = task.contract {
                write_field(w, "  ", "Receives", &contract.receives)?;
                write_field(w, "  ", "Produces", &contract.produces)?;
                write_field(w, "  ", "Verify", &contract.verify)?;
            }
            writeln!(w)?;
        }
        Ok(())
    })
}

// -- Prep --

pub fn prep(text: &str) -> Result<()> {
    let mut w = io::stdout().lock();
    writeln!(w, "{text}")?;
    Ok(())
}
