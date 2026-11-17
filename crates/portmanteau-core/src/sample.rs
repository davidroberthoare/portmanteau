//! A complete demonstration project so the studio opens with real work in it.

use crate::*;

pub fn demo() -> Project {
    let mut p = Project::new("The Salt Road", ProjectKind::Novel);
    p.subtitle = "A novel, with a companion pilot".into();
    p.author = "Ada Marlowe".into();
    p.genre = "Literary mystery".into();
    p.logline = "A mapmaker returns to a drowned harbour town to finish her father's survey and finds the coastline — and her family — was never where the charts said.".into();
    p.daily_target = 1200;

    // Folder order: Draft, Research, Front Matter, Back Matter.
    let manuscript = p.folder_id("Draft").unwrap();
    let part_one = p.add_document(
        Some(manuscript.clone()),
        "Part One · The Tide",
        DocKind::Folder,
        Medium::Prose,
    );
    p.document_mut(&part_one).unwrap().synopsis =
        "Mara returns to finish the survey and learns the town has been quietly erased from the charts."
            .into();

    let chapter_one_text = "The tide went out that morning and did not come back.\n\nMara Venn stood at the end of the harbour wall with her father's brass compass in her fist and watched the sea withdraw past the last black rock. It moved like something embarrassed, sliding away from a mess it had made. Where there had been water at dawn there was now a wide, shining plain of mud, and in it the ribs of boats she had only ever seen as shapes under the surface.\n\nShe had come home to finish his survey. That was the story she told the woman at the ferry office, and the woman had looked at her licence photograph for a long time, as if comparing it with a face she remembered.\n\nThe town had not changed. That was the first lie. The houses still leaned into the wind, the church still kept its one bell, and the Salt Road still ran from the square to the water in a straight line that no one could explain. But the doors had new locks, and the noticeboard carried a planning application in a font that did not belong here.\n\nAt the end of the wall, the compass needle swung once and settled on the east. Mara turned it over. Her father had scratched a date inside the lid: 14 August 1987. The day the harbour drowned, people said, though the water had been leaving for years by then.\n\nA boy was standing on the mud below her, barefoot, holding a red bucket.\n\n“You're the map girl,” he said.\n\n“I'm a surveyor.”\n\n“That's what I said.”\n\nHe pointed at a dark line in the mud, a hundred yards out. It ran straight toward the open sea, and then turned, very slightly, north.\n\n“That's the old road,” he said. “It goes to the other harbour.”\n\nMara looked at her father's chart, still furled under her arm. The other harbour was not on it.\n\nShe climbed down.";

    let chapter_one = p.add_document(
        Some(part_one.clone()),
        "Chapter One · The Cartographer",
        DocKind::Chapter,
        Medium::Prose,
    );
    {
        let doc = p.document_mut(&chapter_one).unwrap();
        doc.set_plain_text(chapter_one_text);
        doc.synopsis = "Mara arrives at low tide, meets a boy who knows a road that is not on her father's chart, and begins to suspect the survey is a confession.".into();
        doc.notes = "Opening should feel like a survey report that keeps losing its nerve. Keep the compass and the red bucket. The boy is not named until chapter four.".into();
        doc.label = "Opening".into();
        doc.status = "Revised".into();
        doc.word_target = 2500;
        doc.pinned = true;
        // Two earlier drafts in the version history.
        let first_draft = "The tide was out when Mara arrived.\n\nShe had not been back to the harbour in eleven years, and the first thing she noticed was the smell: salt, diesel, and something older underneath, like a cellar opened after rain. Her father's compass was in her pocket. She did not take it out.\n\nThe ferry office was closed. A notice taped to the glass said the service would resume when the water did.";
        doc.set_plain_text(first_draft);
        doc.snapshot("First pass · arrival", "Original opening, too much weather.");
        doc.set_plain_text("The tide went out that morning and did not come back.\n\nMara Venn stood at the end of the harbour wall with her father's compass in her fist. The town had not changed. That was the first lie.");
        doc.snapshot("Second pass · the boy", "Cut the ferry office; brought the boy in earlier.");
        doc.set_plain_text(chapter_one_text);
    }

    let chapter_two = p.add_document(
        Some(part_one.clone()),
        "Chapter Two · Low Water",
        DocKind::Chapter,
        Medium::Prose,
    );
    {
        let doc = p.document_mut(&chapter_two).unwrap();
        doc.set_plain_text("By noon the mud had begun to steam.\n\nMara walked the old road with the boy behind her, keeping a distance that was not quite respect and not quite fear. He carried the red bucket. It held a knife, a length of string, and a brass key the size of her thumb.\n\nThe road was made of ballast and crushed shell. It held her weight. Out here, where the chart showed twelve fathoms of water, she could see the walls of gardens, a doorstep, a boot.\n\n“What happened here?” she asked.\n\n“It went out,” the boy said.\n\n“The sea doesn't just go out.”\n\nHe considered this with the patience of someone who had been told otherwise all his life.\n\n“This one did.”\n\nAt the turn, where the road bent north, Mara stopped. A line of iron posts marked the edge of something. She knelt and brushed the mud away with her thumb. Underneath was a plate, riveted to the post, and on the plate was a number: 14.\n\nHer father's chart had a plate 14. It was three miles away, on the other side of the headland.\n\nShe took out her phone. No signal, of course. She took out the compass instead. The needle found east and stayed there, steady as an accusation.");
        doc.synopsis = "The old road leads somewhere the chart insists cannot exist; a numbered plate puts the whole survey in question.".into();
        doc.notes = "The key should not be explained until Part Two. Every object the boy carries must return.".into();
        doc.label = "Development".into();
        doc.status = "Draft".into();
        doc.word_target = 2200;
    }

    let part_two = p.add_document(
        Some(manuscript.clone()),
        "Part Two · The Crossing",
        DocKind::Folder,
        Medium::Prose,
    );
    p.document_mut(&part_two).unwrap().order = 20;
    p.document_mut(&part_two).unwrap().synopsis =
        "Mara follows the old road north and finds the harbour that was taken off the map.".into();

    let chapter_three = p.add_document(
        Some(part_two.clone()),
        "Chapter Three · A Borrowed Name",
        DocKind::Chapter,
        Medium::Prose,
    );
    {
        let doc = p.document_mut(&chapter_three).unwrap();
        doc.set_plain_text("The other harbour had no name on any chart she owned.\n\nIt had a name on the sign above the customs house, but the sign had been painted over twice, and the last coat was the colour of wet slate. Mara could read the older letters only by the shadows they cast in the paint.\n\nShe read: VENN.\n\nHer father had signed every chart in the survey office with his full name, in ink that faded to brown. Here the name was a place.\n\nAn old woman was sitting on the customs steps, shelling peas into a bowl.\n\n“You'll want the key,” the woman said, without looking up.\n\n“I don't have a key.”\n\n“Then you'll want the one the boy has.”\n\nThe woman held out a pea pod, split it with her thumbnail, and let the peas fall.\n\n“Tell him the tide is coming back,” she said. “It always comes back when the map is finished.”");
        doc.synopsis = "The town on the other side of the headland carries Mara's family name, and an old woman knows about the boy's key.".into();
        doc.label = "Reveal".into();
        doc.status = "Draft".into();
        doc.word_target = 2000;
    }

    // ---- Scripts (kept in the Draft folder) -------------------------------
    let cold_harbor = p.add_document(
        Some(manuscript.clone()),
        "Cold Harbor · Pilot",
        DocKind::Scene,
        Medium::TvScript,
    );
    {
        let doc = p.document_mut(&cold_harbor).unwrap();
        doc.synopsis = "A television pilot: a surveyor comes home to a harbour that has been erased from every official chart.".into();
        doc.notes = "60-minute drama. Teaser plus four acts. Cold open on the mud, no music until the title card.".into();
        doc.label = "Script".into();
        doc.status = "Revised".into();
        doc.word_target = 8000;
        doc.set_script_blocks(vec![
            block(ElementKind::Transition, "FADE IN:"),
            block(ElementKind::SceneHeading, "EXT. COLD HARBOR BREAKWATER — DAWN"),
            block(ElementKind::Action, "The sea has gone out. Not a tide: a withdrawal. The harbour lies open like an unfinished sentence, ribs of boats showing in the mud."),
            block(ElementKind::Action, "MARA VENN (30s, salt in her hair, a surveyor's case under one arm) stands at the end of the wall. She is the only person in frame who is not looking at the water."),
            block(ElementKind::Character, "MARA"),
            block(ElementKind::Parenthetical, "(to herself)"),
            block(ElementKind::Dialogue, "The chart says there's a channel here."),
            block(ElementKind::Action, "A BOY (9) sits on the mud below her, barefoot, a red bucket between his knees."),
            block(ElementKind::Character, "BOY"),
            block(ElementKind::Dialogue, "The chart's wrong."),
            block(ElementKind::Character, "MARA"),
            block(ElementKind::Dialogue, "Charts aren't wrong. They're out of date."),
            block(ElementKind::Character, "BOY"),
            block(ElementKind::Dialogue, "Same thing."),
            block(ElementKind::Action, "He tips the bucket. A brass key the size of a thumb falls into the mud. He does not pick it up."),
            block(ElementKind::Transition, "CUT TO:"),
            block(ElementKind::SceneHeading, "INT. HARBOUR OFFICE — DAY"),
            block(ElementKind::Action, "Peeling maps cover every wall. ELIAS ROOK (50s, cardigan, reading glasses on a string) is making tea in a room that has not been used as an office in years."),
            block(ElementKind::Character, "ELIAS"),
            block(ElementKind::Dialogue, "You're Venn's girl."),
            block(ElementKind::Character, "MARA"),
            block(ElementKind::Dialogue, "I'm his surveyor."),
            block(ElementKind::Character, "ELIAS"),
            block(ElementKind::Dialogue, "That's what I said."),
            block(ElementKind::Action, "He sets down two cups. The second one is already full."),
            block(ElementKind::Character, "ELIAS"),
            block(ElementKind::Parenthetical, "(quietly)"),
            block(ElementKind::Dialogue, "He said you'd come back when the water did."),
            block(ElementKind::Action, "Mara looks past him, through the window, at the wide empty harbour."),
            block(ElementKind::Character, "MARA"),
            block(ElementKind::Dialogue, "Then we have a problem."),
            block(ElementKind::Transition, "SMASH CUT TO BLACK."),
            block(ElementKind::Transition, "TITLE CARD: COLD HARBOR"),
        ]);
        doc.snapshot("Read-through draft", "Trimmed the teaser by two pages.");
    }

    let lamp_room = p.add_document(
        Some(manuscript.clone()),
        "The Lamp Room · One act",
        DocKind::Scene,
        Medium::StagePlay,
    );
    {
        let doc = p.document_mut(&lamp_room).unwrap();
        doc.synopsis = "A short stage play: two sisters keep a lighthouse lamp burning for a ship that has already sunk.".into();
        doc.notes = "One set: the lamp room. Two chairs, a table, the lamp. No interval.".into();
        doc.label = "Stage".into();
        doc.status = "Draft".into();
        doc.set_script_blocks(vec![
            block(ElementKind::Act, "ACT ONE"),
            block(ElementKind::Scene, "The lamp room. Night. The lamp is lit, though the glass is cracked."),
            block(ElementKind::StageDirection, "NELL (60s) winds the mechanism. RUTH (50s) watches the sea through binoculars that are held together with tape."),
            block(ElementKind::Character, "RUTH"),
            block(ElementKind::Dialogue, "There's a light out there."),
            block(ElementKind::Character, "NELL"),
            block(ElementKind::Dialogue, "There's always a light."),
            block(ElementKind::Character, "RUTH"),
            block(ElementKind::Dialogue, "Not that one."),
            block(ElementKind::StageDirection, "Nell stops winding. The mechanism ticks down. Silence, except for the sea."),
            block(ElementKind::Character, "NELL"),
            block(ElementKind::Dialogue, "How long have you been looking?"),
            block(ElementKind::Character, "RUTH"),
            block(ElementKind::Dialogue, "Since the Tuesday."),
            block(ElementKind::Character, "NELL"),
            block(ElementKind::Parenthetical, "(gently)"),
            block(ElementKind::Dialogue, "That was nineteen years ago, Ruth."),
            block(ElementKind::StageDirection, "Ruth lowers the binoculars. She does not look at her sister."),
            block(ElementKind::Character, "RUTH"),
            block(ElementKind::Dialogue, "It was a Tuesday."),
            block(ElementKind::StageDirection, "The lamp gutters, then holds."),
            block(ElementKind::Transition, "BLACKOUT."),
        ]);
    }

    // ---- Research ---------------------------------------------------------
    let research_folder = p.folder_id("Research").unwrap();
    let add_research = |p: &mut Project, title: &str, kind: &str, source: &str, body: &str| {
        let id = p.add_document(
            Some(research_folder.clone()),
            title,
            DocKind::Research,
            Medium::Note,
        );
        let doc = p.document_mut(&id).unwrap();
        doc.set_plain_text(body);
        doc.synopsis = body.chars().take(120).collect();
        doc.include_in_export = false;
        p.research.push(ResearchItem {
            id: new_id("res"),
            title: title.into(),
            body: body.into(),
            kind: kind.into(),
            source: source.into(),
            created: now_stamp(),
            attachments: Vec::new(),
            html: String::new(),
        });
    };
    add_research(
        &mut p,
        "Tide tables, 1987–present",
        "Data",
        "Harbour authority archive",
        "Mean low water has fallen 1.4 metres since the survey of 1987. The 1987 chart records a channel of twelve fathoms where the modern soundings show mud and ballast. Two entries in the tide ledger are missing: 13 and 14 August 1987.\n\nNote for chapter two: a surveyor would check the instrument before suspecting the world. Mara checks the world first because she already suspects her father.",
    );
    add_research(
        &mut p,
        "Interview · harbour master",
        "Interview",
        "Recorded at the Salt Road café, 12 March",
        "“The harbour didn't drown. It was taken off the list.” He said this without drama, the way a man says the ferry is late.\n\nHe remembers the boy. He would not say which boy. He remembers a survey party in 1987 with three men and a fourth who did not sign the sheet.\n\nHe asked me not to use his name. I have used it anyway, because the town is small and the archive is smaller.",
    );
    add_research(
        &mut p,
        "Wreck of the Anna Rose",
        "Archive",
        "County records, box 41",
        "A coastal trader, lost on the night of 14 August 1987 with a crew of six. The wreck was never formally located. The insurers paid on the presumption of sinking, not on evidence.\n\nAttached: a photograph of the vessel at the quay, and a hand-drawn scrap of chart showing a channel that does not appear on the printed edition.",
    );
    add_research(
        &mut p,
        "Coastal survey notes",
        "Notes",
        "Mara's field notebook",
        "Surveyors do not believe in ghosts. They believe in benchmarks: fixed points that tell you where you are when everything else moves.\n\nPlate 14 is a benchmark. If plate 14 is in the wrong place, either the plate moved or the map did. Both are possible. Neither is comfortable.\n\nCheck theodolite calibration before chapter three.",
    );

    // ---- Story database ---------------------------------------------------
    let mut add_item = |title: &str, role: &str, description: &str, want: &str, need: &str, flaw: &str, arc: &str, color: u8| {
        let mut item = DatabaseItem::new(ItemKind::Character, title);
        item.notes = format!(
            "Role: {role}\n\nDescription: {description}\n\nWant: {want}\n\nNeed: {need}\n\nFlaw: {flaw}\n\nArc: {arc}"
        );
        item.color = color;
        p.items.push(item);
    };
    add_item(
        "Mara Venn",
        "Protagonist",
        "A coastal surveyor in her thirties. Precise, dry, allergic to being called the map girl.",
        "To finish her father's survey and sell the family house.",
        "To stop treating inheritance as a set of measurements.",
        "She trusts instruments more than people, especially when people are telling the truth.",
        "From mapping the coast to letting it change shape.",
        0,
    );
    add_item(
        "Elias Rook",
        "Ally / keeper of the archive",
        "Harbour master emeritus. Makes tea in an office that has not been an office since 1987.",
        "To keep the town's records intact.",
        "To admit which records he removed.",
        "He confuses preservation with protection.",
        "From custodian of a lie to witness.",
        1,
    );
    add_item(
        "The Boy",
        "Catalyst",
        "Nine years old, barefoot, carries a red bucket and a brass key. May be a memory, may be a neighbour's child.",
        "To be taken seriously.",
        "To be let go.",
        "He knows more than he can say.",
        "He gives Mara the key, then disappears from the story until Part Two.",
        2,
    );
    add_item(
        "Ruth Alder",
        "Sister / keeper of the lamp",
        "Mara's aunt. Has watched the sea every Tuesday for nineteen years.",
        "For the ship to come back.",
        "To mourn a brother who chose the map over the family.",
        "She has made a religion of waiting.",
        "From waiting to remembering.",
        3,
    );

    // ---- Front and back matter -------------------------------------------
    let front = p.folder_id("Front Matter").unwrap();
    let copyright = p.add_document(Some(front.clone()), "Copyright", DocKind::FrontMatter, Medium::Prose);
    p.document_mut(&copyright).unwrap().set_plain_text(
        "Copyright © 2026 Ada Marlowe\n\nAll rights reserved. No part of this book may be reproduced without written permission, except brief quotations in a review.\n\nThis is a work of fiction. Names, characters, places and incidents are products of the author's imagination.",
    );
    let dedication = p.add_document(Some(front.clone()), "Dedication", DocKind::FrontMatter, Medium::Prose);
    p.document_mut(&dedication)
        .unwrap()
        .set_plain_text("For everyone who has ever drawn a coastline from memory.");
    let preface = p.add_document(Some(front.clone()), "Author's note", DocKind::FrontMatter, Medium::Prose);
    p.document_mut(&preface).unwrap().set_plain_text(
        "This book began with a map that was wrong in a way I could not explain. I have tried to be faithful to the coast, and to the people who keep its records.",
    );

    let back = p.folder_id("Back Matter").unwrap();
    let acknowledgements = p.add_document(Some(back.clone()), "Acknowledgements", DocKind::BackMatter, Medium::Prose);
    p.document_mut(&acknowledgements)
        .unwrap()
        .set_plain_text("With thanks to the harbour archivists, the patient readers, and the people of a town that would prefer not to be named.");
    let about = p.add_document(Some(back.clone()), "About the author", DocKind::BackMatter, Medium::Prose);
    p.document_mut(&about)
        .unwrap()
        .set_plain_text("Ada Marlowe is a writer and former coastal surveyor. She lives within sight of the sea, which is not always a comfort.");

    // ---- Session history --------------------------------------------------
    let words = [820, 1140, 420, 1360, 980, 0, 0, 1180, 1510, 640, 1220, 890, 1440, 760];
    let today = chrono::Local::now();
    for (i, w) in words.iter().enumerate() {
        let date = today - chrono::Duration::days((words.len() - 1 - i) as i64);
        p.session_log.push(SessionEntry {
            date: date.format("%Y-%m-%d").to_string(),
            words: *w,
        });
    }

    // ---- Per-format compile settings -------------------------------------
    // The same manuscript finishes differently as a book and as an ebook, so
    // the demonstration project ships with each target already customised.
    let mut pdf = p.settings_for(ExportKind::Pdf);
    pdf.paper = PaperSize::A5;
    pdf.font_size = 11.0;
    pdf.line_spacing = 1.45;
    pdf.header_footer.page_numbers = true;
    pdf.header_footer.running_header = false;
    p.set_settings(pdf.clone());
    p.export = pdf;

    let mut epub = p.settings_for(ExportKind::Epub);
    epub.font_size = 12.0;
    epub.line_spacing = 1.6;
    epub.header_footer.page_numbers = false;
    epub.header_footer.running_header = false;
    p.set_settings(epub);

    let mut fdx = p.settings_for(ExportKind::Fdx);
    fdx.include_title_page = true;
    fdx.include_toc = false;
    fdx.include_chapter_numbers = false;
    p.set_settings(fdx);

    p.checkpoint("First draft checkpoint", "milestone");
    p
}

fn block(kind: ElementKind, text: &str) -> Block {
    Block {
        kind,
        text: text.into(),
    }
}
