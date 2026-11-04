# Stage 6: usable by a beginner (organise by the game, not by files)

The owner tried v0.1.0 as a beginner and found it confusing. Every point below is from that session. The goal from
SPEC 4b stands: **even a beginner can make a ROM hack**. Users think in characters, karts, tracks, menus, items,
music and text, never in NCGR/NCLR/NSBMD/BMG. Keep every engine; change how it's presented.

**How to work:** breadth first. Each item works end to end before polish. Keep the checks green, leave changes
uncommitted, and use MKDS_ROM (docs/STAGE4.md) for retail checks, reading it in place.

## A. Home: "What do you want to make?"
After opening a ROM or project, show cards: Replace a character, Replace a kart, Make a track, Change music,
Recolour or redraw a menu / HUD / item, Change text and names, Make a cup. Each opens the matching wizard or guided
flow. A "Recent" strip shows what the user opened and edited last.

## B. Game browser (replaces Explorer as the default)
- A tree by meaning, built from `mkds-data` and the research docs:
  - **Characters:** each has race model, far-away model, menu model, icons, animations and voice;
  - **Karts;**
  - **Tracks:** by course name, grouped by cup;
  - **Menus & HUD;**
  - **Items;**
  - **Music & sounds;**
  - **Text.**
- Every entry says, in plain words, where the player sees it. Examples:
  - "Mario, seen when you race as Mario" (`P_MR`);
  - "Mario far away / as an opponent at a distance" (`E_MR`);
  - "Mario in the character select menu".
- Thumbnails: rendered models, images with their palettes, and course top-down previews.
- The raw file Explorer moves under **Advanced** (or behind a setting).
- **Navigation:**
  - back/forward buttons;
  - open items stay as tabs;
  - "Recently opened";
  - a search box over the friendly names that also matches file paths.

## C. Text
- Show **English (US) by default**, with a language switch (es/fr/ge/it are separate files in the USA ROM). The owner
  saw Spanish text and thought it was garbage.
- Label message groups by where they appear: kart names, course names, cup names, menu, results, and so on.
- Show control codes as friendly chips (colour, button icon, player name).
- Show a preview in the game font where possible.
- Search across all text.

## D. Images (Texture & 2D, made friendly)
- **Image gallery:** every 2D screen, icon and texture rendered with the right palette automatically. Pair NCGR, NCLR,
  NSCR and NCER using what the files reference and the research, with no manual "choose a palette". Unpairable
  leftovers go to Advanced.
- **Swap a colour:**
  - click a colour in the image or palette strip and pick a new one;
  - say how many images share that palette ("this palette is also used by 3 other images") and preview them.
- **Redraw:**
  - paint directly with simple tools;
  - or Export PNG, edit anywhere, then Import PNG with automatic conversion and a before/after preview.
- Never require format names. Keep the technical details in a collapsible "Details" section.

## E. Audio
- The owner reported that "Preview" did nothing. Find out why (silent bank, wrong sequence, mixer not started, or a
  one-tick clip) and fix it.
- Music is listed by name: course themes from the music table, plus menu and results tunes.
- Preview plays the whole song with its real bank, with play/stop, a position bar and duration.
- Sound effects are listed by group with play buttons.
- "Replace this song" and "Replace this sound" open the existing import flows.

## F. Models and real downloaded files
- Wizards must accept typical downloaded models:
  - tens of thousands of triangles;
  - hundreds of bones;
  - matrix bones;
  - bind poses that differ from the rest pose.

  The maintainer added `model_import::import_static` and matrix decomposition; build on them.
- **Speed:** decimating 31,600 triangles took over 15 minutes. It must finish in under 10 s (spatial hashing,
  heap-based edge collapse, or a vertex-clustering pre-pass before quadric simplification).
- Add a **synthetic** stress fixture generated in code (never a third-party model): a 30k-triangle humanoid with
  200+ bones, some matrix bones, and a mismatched bind pose. The character, kart and track wizards must each handle
  it in under 30 s.
- Imported track floors need a default visible colour or texture (today an untextured map renders black in game).
  Generate the minimap image (`Map2D`); today it's blank in game.

## G. Saving, and room for experts
- A visible **Save** button in the top bar and **Ctrl+S**. Edits already persist; make that obvious:
  - a "Saved" / "Unsaved changes" indicator;
  - File > Save, Save As (copy the project to a new folder) and Open Recent;
  - a warning when closing with work that isn't exported yet.
- **Beginner first, expert available.** Defaults are the friendly views and guided flows; every existing expert tool
  stays (raw Explorer, file pairing, format details, numeric transforms, collision painting, Code). Put them under
  an "Advanced" toggle or a "Show details" area, never removed.
- **Smart defaults everywhere:** auto-pick the right files, sizes, palettes and budgets; show a plain-language reason
  plus a one-click fix when something won't fit on a DS.

## H. Done means
- Checks green, plus Playwright flows for each card on the Home screen (synthetic data).
- Screenshots of Home, the Game browser, Text, the Image gallery with Swap a colour, Audio and a wizard.
- docs/STATUS.md updated with what a beginner can now do, step by step.
