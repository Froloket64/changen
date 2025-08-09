use git2::{Commit, Repository, Revwalk};

use std::env;
use std::fs;
use std::io;

mod conventional_commits;
use conventional_commits::{CommitDesc, parse_commit_msg};

mod fmt;
use fmt::MdFormatter;

// NOTE: Question-mark comments show where I'm confused about returned type
// safety (why is there a `Result`, can it be safely `.unwrap`ped, etc.)
fn main() -> io::Result<()> {
    let filename = env::args().skip(1).next().unwrap_or_else(|| {
        println!("warning: not output file was specified, using `CHANGELOG.md`");

        "CHANGELOG.md".into()
    });

    // HACK?
    let repo = Repository::open(".").expect("failed to find repository");
    let mut file = fs::File::create(filename)?;
    let mut fmt = MdFormatter::new(&mut file, &repo);

    let head = repo.head().expect("failed to determine HEAD");
    let latest_commit_oid = head.target().expect("failed to get latest commit");
    let mut tagged_commits = find_tagged_commits(&repo).unwrap(); // Result?
    let mut commits = repo.revwalk().unwrap(); // Result?

    commits.push(latest_commit_oid).unwrap(); // ?

    let commit_descriptions = commit_descriptions(commits, &repo);

    // TODO: Grouping
    // commit_descriptions.sort_by_key(|commit| commit.msg.tag().unwrap_or("").to_string());

    fmt.start_changelog()?;

    // HACK?
    let mut rc = Ok(());

    commit_descriptions.into_iter().for_each(|commit| {
        match tagged_commits.last() {
            Some((tag_name, tagged_commit)) if tagged_commit.id() == commit.id() => {
                fmt.write_tag(tag_name.strip_prefix("refs/tags/").unwrap())
                    .unwrap();
                tagged_commits.pop();
            }
            _ => (),
        }

        fmt.write_commit(&commit).err().map(|e| rc = Err(e));
    });

    rc?;

    Ok(())
}

fn find_tagged_commits(repo: &Repository) -> Result<Vec<(String, Commit)>, git2::Error> {
    let mut commits = vec![];

    repo.tag_foreach(|oid, name| {
        let commit = repo.find_commit(oid).unwrap();

        // HACK?
        commits.push((String::from_utf8(name.to_vec()).unwrap(), commit));

        true
    })
    .map(|_| commits)
}

fn commit_descriptions(commits: Revwalk, repo: &Repository) -> Vec<CommitDesc> {
    let mut descriptions = vec![];

    commits.for_each(|oid| {
        let commit_oid = oid.unwrap(); // safe?
        let commit = repo.find_commit(commit_oid).unwrap(); // safe?
        // turns out this fails sometimes :(
        let msg_raw = match commit.message() {
            Some(x) => x,
            None => return,
        };

        let msg = parse_commit_msg(msg_raw);
        let commit_desc = CommitDesc::new(commit_oid, msg);
        let commit_desc = match commit.author().name() {
            None => commit_desc,
            Some(author) => commit_desc.with_author(author.into()),
        };

        descriptions.push(commit_desc);
    });

    descriptions
}
