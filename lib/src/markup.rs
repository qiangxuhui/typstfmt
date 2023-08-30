use super::*;
use typst_syntax::ast::AstNode;

#[instrument(skip_all)]
pub(crate) fn format_content_blocks(
    parent: &LinkedNode,
    children: &[String],
    ctx: &mut Ctx,
) -> String {
    let mut res = String::new();
    let markup = parent
        .cast_first_match::<typst_syntax::ast::Markup>()
        .unwrap_or_default();
    let first_space = markup.as_untyped().children().next();
    let spaced = first_space.is_some_and(|x| x.kind() == Space);

    for (s, c) in children.iter().zip(parent.children()) {
        match c.kind() {
            RightBracket if spaced => {
                let space_type = if first_space.unwrap().text().contains('\n') {
                    '\n'
                } else {
                    ' '
                };
                if !res.ends_with(space_type) {
                    while res.ends_with('\n') || res.ends_with(' ') {
                        res.replace_range(res.len() - 1..res.len(), "");
                    }
                    res.push(space_type);
                }
                ctx.push_raw_in(s, &mut res)
            }
            _ => ctx.push_raw_indent(s, &mut res),
        }
    }
    res
}

// break lines so they won't go over max_line_length
#[instrument(skip_all)]
pub(crate) fn format_markup(parent: &LinkedNode, children: &[String], ctx: &mut Ctx) -> String {
    let mut res = String::new();
    let mut skip_until = None;
    let parent_is_list = [Some(EnumItem), Some(ListItem)].contains(&parent.parent_kind());

    for (idx, (s, node)) in children.iter().zip(parent.children()).enumerate() {
        match node.kind() {
            _ if skip_until.is_some_and(|skip| idx <= skip) => {}
            Space => {
                // todo, hack to get enums and lists to work
                if [Some(EnumItem), Some(ListItem)].contains(&parent.parent_kind()) {
                    ctx.push_raw_in(s, &mut res);
                    continue;
                }

                if idx == 0
                    || idx == children.len()
                    || [Some(Text), Some(Parbreak)].contains(&node.next_sibling_kind())
                    || ![Some(Text), Some(Parbreak)].contains(&node.prev_sibling_kind())
                    || node.prev_sibling_kind() == Some(Linebreak)
                {
                    ctx.push_raw_in(s, &mut res);
                }
            }
            Text => {
                // We eat all the following nodes if they're in `[Space, Text, Emph, Strong, Label, Ref]`
                // then we format ourselves breaking or spacing.
                skip_until = Some(idx);
                let mut this = node;
                let mut add = s.to_string();
                loop {
                    let next = utils::find_next(&this, &|_| true);
                    match next.as_ref() {
                        Some(next) => {
                            if ![Space, Text, Emph, Strong, Label, Ref].contains(&next.kind()) {
                                break;
                            }
                            if next.kind() == Space
                                && [EnumItem, ListItem]
                                    .map(Some)
                                    .contains(&next.next_sibling_kind())
                            {
                                break;
                            }
                        }
                        None => break,
                    }

                    *skip_until.as_mut().unwrap() += 1;
                    this = next.unwrap();
                    match this.kind() {
                        Space => add.push(' '),
                        _ => add.push_str(&children[skip_until.unwrap()]),
                    }
                }
                let split = add.split(' ').filter(|x| !x.is_empty()).collect_vec();
                for word in split.iter() {
                    if utils::max_line_length(word)
                        + 1 // the space we're adding
                        + utils::max_line_length(res.lines().last().unwrap_or(""))
                        <= ctx.config.max_line_length
                    {
                        res.push_str(word);
                        res.push(' ');
                    } else {
                        if res.ends_with(' ') {
                            res = res[..res.len() - 1].to_string();
                        }
                        res.push('\n');
                        res.push_str(word);
                        res.push(' ');
                    }
                }

                // we don't want to end with a space nor to see `don 't`
                if res.ends_with(' ') && this.next_sibling().is_none()
                    || [Some(Text), Some(SmartQuote)].contains(&this.next_sibling_kind())
                {
                    res = res[..res.len() - 1].to_string();
                }

                if parent_is_list && this.next_sibling_kind().is_none() {
                    res.push('\n');
                }
            }
            _ => {
                ctx.push_raw_in(s, &mut res);
            }
        }
    }

    ctx.lost_context();
    res
}
