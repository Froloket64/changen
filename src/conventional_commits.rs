use git2::Oid;
use peeking_iter::peeking::*;

pub struct CommitDesc {
    oid: Oid,
    msg: MsgDesc,
    author: Option<String>,
}

impl CommitDesc {
    pub fn new(oid: Oid, msg: MsgDesc) -> Self {
        Self {
            oid,
            msg,
            author: None,
        }
    }

    pub fn with_author(self, author: String) -> Self {
        Self {
            author: Some(author),
            ..self
        }
    }

    pub fn id(&self) -> Oid {
        self.oid
    }

    pub fn message(&self) -> &MsgDesc {
        &self.msg
    }

    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }
}

#[derive(Debug, PartialEq)]
pub struct MsgDesc {
    msg: String,
    conv_tag: Option<String>,
    conv_scope: Option<String>,
    is_breaking: bool,
}

impl MsgDesc {
    pub fn just_msg(msg: String) -> Self {
        Self {
            msg,
            conv_tag: None,
            conv_scope: None,
            is_breaking: false,
        }
    }

    pub fn with_tag(self, tag: String) -> Self {
        Self {
            conv_tag: Some(tag),
            ..self
        }
    }

    pub fn with_tag_scope(self, tag: String, scope: String) -> Self {
        Self {
            conv_tag: Some(tag),
            conv_scope: Some(scope),
            ..self
        }
    }

    pub fn breaking(self) -> Self {
        Self {
            is_breaking: true,
            ..self
        }
    }

    pub fn msg(&self) -> &str {
        &self.msg
    }

    pub fn tag(&self) -> Option<&str> {
        self.conv_tag.as_deref()
    }

    pub fn scope(&self) -> Option<&str> {
        self.conv_scope.as_deref()
    }
    
    pub fn is_breaking(&self) -> bool {
        self.is_breaking
    }
    
    pub fn is_conventional(&self) -> bool {
        self.tag().is_some()
    }
}

pub fn parse_commit_msg(input: &str) -> MsgDesc {
    let mut it = input.chars().to_peeking();

    let tag: String = it.next_while(|c| c.is_alphabetic()).into_iter().collect();

    if tag.is_empty() {
        return MsgDesc::just_msg(it.collect::<String>());
    }

    match it.peek() {
        None => return MsgDesc::just_msg(tag),
        Some(c) => match c {
            '(' => {
                it.advance_to_peeked();

                let scope: Vec<char> = it.next_while1(|c| *c != ')');
                let scope: Option<String> = if scope.is_empty() {
                    None
                } else {
                    Some(scope.into_iter().collect())
                };

                match it.peek() {
                    Some('!') => match it.peek() {
                        Some(':') => match parse_conventional_sep(&mut it) {
                            Some(msg) => MsgDesc {
                                msg,
                                conv_tag: Some(tag),
                                conv_scope: scope,
                                is_breaking: true,
                            },
                            None => MsgDesc::just_msg(tag + &it.collect::<String>()),
                        },
                        _ => {
                            it.rewind_peeking();

                            MsgDesc::just_msg(tag + &it.collect::<String>())
                        }
                    },
                    Some(':') => match parse_conventional_sep(&mut it) {
                        Some(msg) => MsgDesc {
                            msg,
                            conv_tag: Some(tag),
                            conv_scope: scope,
                            is_breaking: false,
                        },
                        None => MsgDesc::just_msg(tag + &it.collect::<String>()),
                    },
                    _ => {
                        it.rewind_peeking();

                        MsgDesc::just_msg(tag + &it.collect::<String>())
                    }
                }
            }
            '!' => match it.peek() {
                Some(':') => match parse_conventional_sep(&mut it) {
                    Some(msg) => MsgDesc {
                        msg,
                        conv_tag: Some(tag),
                        conv_scope: None,
                        is_breaking: true,
                    },
                    None => MsgDesc::just_msg(tag + &it.collect::<String>()),
                },
                _ => {
                    it.rewind_peeking();
                    MsgDesc::just_msg(tag + &it.collect::<String>())
                }
            },
            ':' => match parse_conventional_sep(&mut it) {
                Some(msg) => MsgDesc {
                    msg,
                    conv_tag: Some(tag.to_string()),
                    conv_scope: None,
                    is_breaking: false,
                },
                None => MsgDesc::just_msg(tag + &it.collect::<String>()),
            },
            _ => MsgDesc::just_msg(tag + &it.collect::<String>()),
        },
    }
}

fn parse_conventional_sep<I: Iterator<Item = char> + Clone>(
    it: &mut PeekingIter<I>,
) -> Option<String> {
    // Required whitespace after ':'
    match it.peek() {
        Some(c) if c.is_whitespace() => it.advance_to_peeked(),
        _ => {
            it.rewind_peeking();

            return None;
        }
    }

    Some(it.collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::parse_commit_msg;
    use super::MsgDesc;

    #[test]
    pub(crate) fn message_parsing() {
        // Positive
        assert_eq!(
            parse_commit_msg("feat: lorem ipsum"),
            MsgDesc::just_msg("lorem ipsum".into()).with_tag("feat".into())
        );
        assert_eq!(
            parse_commit_msg("feat(design): lorem ipsum"),
            MsgDesc::just_msg("lorem ipsum".into()).with_tag_scope("feat".into(), "design".into())
        );
        assert_eq!(
            parse_commit_msg("feat!: lorem ipsum"),
            MsgDesc::just_msg("lorem ipsum".into())
                .with_tag("feat".into())
                .breaking()
        );
        assert_eq!(
            parse_commit_msg("feat(design)!: lorem ipsum"),
            MsgDesc::just_msg("lorem ipsum".into())
                .with_tag_scope("feat".into(), "design".into())
                .breaking()
        );
        assert_eq!(parse_commit_msg("feat(): lorem ipsum"), MsgDesc::just_msg("lorem ipsum".into()).with_tag("feat".into()));

        // Negative
        assert_eq!(
            parse_commit_msg("lorem ipsum"),
            MsgDesc::just_msg("lorem ipsum".into())
        );
        assert_eq!(parse_commit_msg("lorem ipsum:"), MsgDesc::just_msg("lorem ipsum:".into()));
        assert_eq!(parse_commit_msg("feat:lorem ipsum"), MsgDesc::just_msg("feat:lorem ipsum".into()));
        assert_eq!(parse_commit_msg("():lorem ipsum"), MsgDesc::just_msg("():lorem ipsum".into()));
    }
}
