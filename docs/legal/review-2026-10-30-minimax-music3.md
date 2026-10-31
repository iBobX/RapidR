# Legal review, 2026-10-30: MiniMax-Music3 (local) as the music bed of RapidR's videos

Date: 2026-10-30. Reviewer: RapidR's conservative legal reviewer (agent). **Not legal advice** (US/EU practice; no lawyer consulted; Robert has no lawyer budget, so the conservative choice is preferred throughout).

**Question.** May Ruta Internet S.R.L. (Uruguay; owner of RapidR, MIT-licensed, free, funded by GitHub Sponsors / Buy Me a Coffee) use background music generated **locally** with the open-weight model MiniMax-Music3 (Hugging Face `MiniMaxAI/MiniMax-Music3`; local 8-bit MLX build `ddalcu/MiniMax-Music3-MLX-Serve-8bit`, served by the MLX-Serve app) as the music bed of its narrated tutorial / launch videos, published on YouTube, GitHub (release pages, Discussions), X, groups.io and rapidr.dev? The music is only in the videos: never shipped in RapidR, never offered to users.

## Verdict: GO WITH CONDITIONS

- **The licence allows it.** It is an MIT-style grant plus three conditions that bite here: credit the name "MiniMax-Music3" prominently, disclose that the content is machine-generated, and follow the Acceptable Use Policy (Exhibit A). The $20 million revenue threshold is far away. Nothing in the licence flows down to RapidR's users, because the audio is never shipped.
- **The residual risk is not the licence, it is the output.** The licence is silent on who owns outputs, MiniMax gives no warranty and no IP indemnity, and tells you to "assume any and all risks" for outputs (quoted below). The model card says nothing about training data. For a quiet instrumental bed under a tutorial the practical risk is low (at worst an automated Content ID claim or a takedown, not a lawsuit), but it cannot be brought to zero. **Apple Loops under Robert's Logic Pro licence remains the lower-risk default**; MiniMax-Music3 is allowed when Robert wants variety, under the conditions in section 6. If a video is the flagship of a launch, use Apple Loops, or do the pre-publication check in condition 7 first.
- **Credit line** (end card, and the video descriptions): `Music: MiniMax-Music3 (AI-generated, instrumental, run locally)` / ES `Música: MiniMax-Music3 (generada con IA, instrumental, en local)`. Full wording and placement in section 6.

## 1. What was read (all on 2026-10-30, about 21:19 local / 00:19 UTC on 2026-10-31)

| Source | URL | Notes |
|---|---|---|
| Licence, upstream | https://huggingface.co/MiniMaxAI/MiniMax-Music3/raw/main/LICENSE | 7,373 bytes, 127 lines, SHA-256 `b21d12df2adae59dad3fcf80c1d81492654c662342f497d9a9770198c9317e58`. Title "MiniMax-Music3 COMMUNITY LICENSE", "Copyright (c) 2026 MiniMax". Read in full, including Exhibit A. Repo revision at the time: `1b525ccb685ae809f33027c51c351eeb5833cf72` (2026-10-26). The licence was last reformatted on 2026-08-11 (commit "docs: format community license"). |
| Licence, MLX build | https://huggingface.co/ddalcu/MiniMax-Music3-MLX-Serve-8bit/raw/main/LICENSE | **Byte-identical** (same SHA-256). Repo revision `8ca48eb3c83150225279bbee8de0a059636af499` (2026-08-13). |
| Model card, upstream | https://huggingface.co/MiniMaxAI/MiniMax-Music3 (raw README read) | No licence field in the metadata; only a "LICENSE" badge linking to the LICENSE file. No section on licence, training data, output ownership, attribution, watermarking or responsible use. Contact: model@minimax.io. |
| Model card, MLX build | https://huggingface.co/ddalcu/MiniMax-Music3-MLX-Serve-8bit | Metadata: `license: other`, `license_name: minimax-music3-community-license`, `license_link: LICENSE`, `base_model_relation: quantized`. Its only licence sentence: "Weights are covered by the MiniMax-Music3 Community License (see `LICENSE`, acceptable-use policy included as Exhibit A)." No extra terms. |
| MLX-Serve app | https://github.com/ddalcu/mlx-serve (LICENSE raw) | MIT, "Copyright (c) 2026 David Dalcu", plus a note that bundled third-party parts (Apache-2.0, BSD-3-Clause, MIT) keep their own licences. It is a local tool; its licence says nothing about what it outputs. |
| Upstream GitHub README | https://github.com/MiniMax-AI/MiniMax-Music3 | Nothing on training data, licensing or responsible use beyond the LICENSE badge. |
| HF discussions | https://huggingface.co/MiniMaxAI/MiniMax-Music3/discussions/26 and /27 | #27 asks MiniMax to add the full texts of the Apache-2.0 / MIT licences it mentions (no reply). #26 asks whether code is open source (no staff reply). Nothing on commercial use or outputs. |
| Logic Pro licence | `/Applications/Logic Pro.app/Contents/Resources/Logic Pro License Agreement.pdf` (Logic Pro 12.4 on Robert's Mac), section 2.C "Sample Content" | Read from the installed copy. |
| Apple support article | https://support.apple.com/en-us/101908 (dated 3 March 2025) | Same Sample Content wording; no mention of credit. |
| US Copyright Office | https://www.copyright.gov/ai/Copyright-and-Artificial-Intelligence-Part-2-Copyrightability-Report.pdf (January 2025) | Conclusions page read. |

## 2. The MiniMax-Music3 Community License, point by point

Short quotes from the licence file (the licence itself allows copying it: "to use, copy, modify, merge, publish, distribute, sublicense").

**Commercial use allowed?** Yes, conditionally. There is no non-commercial clause. The grant is free of charge to "any person" and covers the weights, config, inference code and documentation ("the Software"). The licence then has a section called "Commercial Terms" that regulates, but presupposes, commercial use.

**Attribution (clause 3.1).** "You shall prominently display “MiniMax-Music3” on the user interface of commercial product or service that uses the Software." Points for RapidR:
- The wording is the exact string `MiniMax-Music3` (hyphen, no space; the model card writes "MiniMax Music 3", the licence writes "MiniMax-Music3": use the licence's form).
- Is a free video a "commercial product or service"? Unclear (Ruta is a company, RapidR is free, the videos promote it and sit next to a donation call; a monetised YouTube channel would make it clearer). A video has no "user interface" in the literal sense; the closest things are the frames themselves and the page that presents the video (YouTube page, rapidr.dev page). The conservative reading costs one line of text, so treat it as applying: credit on screen and in the page text. See section 6 for the prominence standard.

**AI-generated content disclosure.** Exhibit A item 11: no "Use to generate or disseminate information (including images, code, posts, or articles) in or to any public environment (including via bot tweets or similar means) without clearly and prominently disclosing that such information and/or content is machine-generated". Music is not in the list of examples, but the sentence says "information and/or content" and the list is "including", so a published video's music bed is conservatively inside it. YouTube, X, GitHub, groups.io and rapidr.dev are all public environments. Disclose clearly and prominently (on screen, and above "Show more" in descriptions).

**Revenue threshold (clause 3.2).** Separate prior written authorisation from MiniMax (api@minimax.io, subject "MiniMax-Music3 licensing - authorization request") "if the aggregate yearly revenue generated from such products and services provided by you and/or any of your affiliates exceeds 20 million US dollars (or equivalent in other currencies)". Far from RapidR's sponsorship income. Not an issue; note it for the day RapidR grows.

**Use-case restrictions (Exhibit A, 19 items).** Checked against software promotion, education and minors:
- Software promotion: none of the 19 items forbids promoting software or making tutorials.
- Education: item 13 forbids "high-risk automated decisions in critical domains" and lists education; a music bed takes no decision about any person, so it is not touched.
- Minors: item 5, "Use in any manner that exploits or harms, or intends to exploit or harm, minors". Beginner tutorials, even for schools, do not exploit or harm minors; not touched.
- Items that need care in how the track is made: 1 (no infringement of third-party IP rights: see risk in section 4), 12 (no impersonating a person without consent: the model makes "expressive vocals"; keep the beds instrumental and never name a real artist in a prompt), 14 ("violates or disregards the social, ethical, or moral standards of other countries or regions": vague; calm instrumental music is the safe end of it), 4 (do not bypass MiniMax's safeguards).
- Item 18 (military) and 19 (unlicensed professional activity): not relevant.
- Clause 4 (safeguards when you "provide or make available to any third party a product, service, or hosted service that permits the generation of outputs"): **not triggered**, because RapidR offers nobody a way to generate music with the model.
- The AUP can change: "MiniMax reserves the right to update this Acceptable Use Policy from time to time", and the current text says "Last revised: August [6], 2026" with the square brackets still in it (it reads like a draft date). So: keep a dated copy, re-check the hash (section 6).

**Who owns outputs?** The licence says **nothing**. No clause assigns outputs to the user, none reserves them for MiniMax, and MiniMax claims nothing in them. The grant of rights is about the Software; outputs appear only in the risk sentence below. Separately, under US law (Copyright Office, Part 2 report, January 2025) copyright "does not extend to purely AI-generated material" and "prompts do not alone provide sufficient control". So Ruta almost certainly **cannot own copyright in the raw output** in the US; the EU/UY position is not settled and was not checked. Practical meaning: Ruta cannot stop others from reusing the track; it must not register it with Content ID or a distributor, and must not claim exclusivity. The video as a whole still carries Ruta's human authorship (screen recordings, script, edit, selection and arrangement of the music).

**Indemnity and liability (in the AS-IS paragraph).**
- Risk on you: "YOU SHALL BE SOLELY RESPONSIBLE FOR DETERMINING THE APPROPRIATENESS OF USING, REPRODUCING, MODIFYING, PERFORMING, DISPLAYING OR DISTRIBUTING ANY OF THE SOFTWARE OR OUTPUTS AND ASSUME ANY AND ALL RISKS ASSOCIATED WITH YOUR OR A THIRD PARTY'S USE OR DISTRIBUTION OF ANY OF THE SOFTWARE OR OUTPUTS". This is the decisive sentence: MiniMax gives **no IP indemnity and no warranty** for outputs; any third-party claim about the output is Ruta's.
- Indemnity by you: "YOU SHALL INDEMNIFY AND HOLD HARMLESS MINIMAX FROM AND AGAINST ANY AND ALL LOSSES ... ARISING OUT OF OR RELATING TO ANY OF YOUR BREACH OF THIS LICENSE." Limited to your breach of the licence (for example a missing credit that costs MiniMax something); the realistic exposure is small, but it is one more reason to meet the credit and disclosure conditions.

**Termination.** No termination clause. The permission is "subject to the following conditions", so breaching a condition would end the permission for the breaching use (as with MIT; not settled law). Practical rule: keep the conditions, and if one is missed in a published video, fix the video's text or replace the track.

**Governing law / venue.** None stated. The licensor is only "MiniMax" (no legal entity named in the licence); press reports describe MiniMax as Shanghai-based. Any dispute would raise conflict-of-law questions that Ruta could not afford to fight, which is another reason for the low-risk conditions below.

**Derivatives and quantized builds.** The grant covers "modify, merge, publish, distribute, sublicense" and condition 1 requires the licence notice in all copies. The ddalcu MLX build is declared `base_model_relation: quantized`, ships the byte-identical LICENSE, and its card points to it, so it is a licence-conforming derivative carrying the same licence. No extra terms from the converter. (MLX-Serve itself is MIT; nothing in it touches the audio.)

**Third-party parts named in the licence.** "fine-tuned from Qwen3-8B" (Apache-2.0), DiT-2B "modified from the Stable Audio code" (MIT, code, not Stability's weights), VAE "modified from the DAC code" (MIT). These are the model's own upstream licences; they ask nothing of a user who only generates audio and ships none of the model.

**Flows to RapidR's users?** No. The model, its weights and its audio are in no RapidR crate, installer, example, Help page or Studio asset. Two things to avoid so it stays true: (a) if the videos are ever published under a Creative Commons licence, carve the music out ("except the music, which is not licensed for reuse"), because Ruta may hold no copyright in it and cannot license it; (b) if a track is ever put into RapidR (a startup sound, an example in `examples/media/`) or RAI ever offers music generation, that is a new review: clause 3.1 would then literally apply to a "user interface" and clause 4 to a generating service.

## 3. Rules that touch this beyond the licence

- **YouTube.** Its altered/synthetic-content question was announced in November 2023 (https://www.loeb.com/en/insights/passle/2023/11/youtube-to-crack-down-on-deepfake-music-and-add-generative-ai-disclosure-requirements): creators must disclose realistic synthetic content, with penalties for hiding it; the current help-page wording was **not** re-read (search returned only 2023 coverage). Robert already answers "Yes" for the AI narration (review 2026-10-30 demo-mode, S5), which also covers the music. Keep answering Yes while the music is AI-generated, and set the audience honestly ("not made for kids" unless a video really is).
- **EU AI Act Art. 50.** As in the demo-mode review: the machine-readable marking duty is the generator provider's; the deployer's disclosure duty is for deep fakes (content resembling real people or events that could pass as authentic). An instrumental bed is not one. The on-screen credit here is the licence's duty (item 11) and transparency, not a proven AI Act duty. Applicability dates and later amendments were not re-checked.
- **Trademarks.** "MiniMax" and "MiniMax-Music3" appear as plain text to say which model made the music (nominative use); no logos, no wording that suggests endorsement.
- **Restricted parties / trade compliance.** Clause 2 says use must "comply with applicable laws and regulations (including trade-compliance laws and regulations)". A web search found no report that MiniMax is on the BIS Entity List or the OFAC SDN list; I could not check the primary lists (BIS Federal Register notices, https://sanctionssearch.ofac.treas.gov/). Using free weights locally from Uruguay is low-risk, but see "Could not verify".

## 4. Risk specific to the output (why Apple Loops stays the default)

- **Training data is undisclosed.** Neither model card nor the GitHub README says what the music model was trained on. AI-music training is being litigated (labels v. Suno and Udio; a class action over Google's Lyria 3; see the search results in section 8). I found **no** report of a music suit against MiniMax. MiniMax is a defendant in a **video** training case, Disney v. MiniMax, C.D. Cal. 2:25-cv-08768, filed 16 September 2025, where press in May 2026 reported its motion to dismiss was not granted (secondary source, not checked against the docket). That does not prove anything about Music3, but it means MiniMax is not a provider whose data provenance can be assumed clean.
- **Memorisation / sound-alike.** A generator can occasionally reproduce or resemble existing music. For an instrumental pad or lo-fi loop under narration the chance is small, but if it happens the licence puts the loss on Ruta (quoted above).
- **Content ID.** The realistic worst case is an automated claim or a takedown on YouTube, which would hurt a launch video far more than it would hurt Ruta in court. Apple Loops can also draw an occasional false claim (from memory), but there Apple's licence is a clear defence.
- **No exclusivity.** Anyone can generate similar music with the same weights, and the output is probably not protectable: do not assert otherwise.

## 5. Comparison with Apple Loops under Robert's Logic Pro licence

Logic Pro License Agreement, section 2.C "Sample Content" (Logic Pro 12.4): Sample Content "may be used on a royalty-free basis to create your own original soundtracks for your audio and video projects"; you "may broadcast, publish, display, and/or distribute your own soundtracks that were created using the Sample Content"; but "individual Sample Content assets may not be commercially or otherwise distributed on a standalone basis", nor repackaged as "music beds" and the like. It also says Apple may modify or remove Sample Content, and bars using Sample Content "for training, testing, or evaluating, any machine learning or artificial intelligence models, tools, algorithms, sound generators". No credit is required. Apple's support article 101908 (3 March 2025) repeats the same wording and also mentions no credit.

| | MiniMax-Music3 (local MLX, 8-bit) | Apple Loops / Logic Pro Sample Content |
|---|---|---|
| Cost | free; Robert's Mac time | already paid (Logic Pro licence) |
| Commercial use | yes (clause 3, with conditions) | yes, royalty-free "original soundtracks" for audio and video projects |
| Credit | **required**: "MiniMax-Music3" prominently | none required |
| AI disclosure | **required** (Exhibit A item 11); also YouTube/X questions | not an AI output (YouTube answer follows the narration anyway) |
| Who owns it | nobody clearly; probably not copyrightable; no exclusivity | Apple/its licensors own the loops; Ruta owns its arrangement and the finished soundtrack |
| IP risk on the sound | undisclosed training data; user "assumes any and all risks"; no indemnity | clear provenance; Apple's licence is the grant; very low |
| Standalone release | not forbidden by the licence, but pointless and risky (no copyright, Content ID) | forbidden for individual assets; the finished soundtrack inside a video is allowed |
| Terms can change | Exhibit A "may be updated from time to time" | Apple can modify or remove Sample Content; each Logic version has its own text |
| Can feed an AI model | n/a | forbidden (do not use loops as reference for any generator) |
| Variety / uniqueness | high, any style, regenerate by seed | fixed library; others use the same loops |
| Effort to stay legal | credit + disclosure + log + pre-check | almost none (never release the audio alone) |

Preview 4's video used Apple Loops, which fits the licence: a finished soundtrack inside a video, nothing released alone.

## 6. Conditions (what makes the GO safe)

1. **Keep a dated copy of the licence.** Save the upstream LICENSE as `docs/legal/minimax-music3-LICENSE-2026-10-30.txt` (7,373 bytes, SHA-256 `b21d12df2adae59dad3fcf80c1d81492654c662342f497d9a9770198c9317e58`, identical in the MLX build) and note the two revisions above. Before each new generation session run `curl -sL https://huggingface.co/MiniMaxAI/MiniMax-Music3/raw/main/LICENSE | shasum -a 256`; if the hash differs, stop and re-review (the AUP is updatable).
2. **Credit, exactly this string and on screen.**
   - End card, as one of the credit lines, same size as the other credit lines or larger, high contrast, on screen at least 4 seconds, not in a corner overlay:
     - EN: `Music: MiniMax-Music3 (AI-generated, instrumental, run locally)`
     - ES: `Música: MiniMax-Music3 (generada con IA, instrumental, en local)`
   - Title card small line (extends the demo-mode review's M2 line): EN `AI-generated narration and music` / ES `Narración y música generadas con IA`.
   - Video descriptions: the same Music line (in the pipeline it is `script.musicCredit` = `MiniMax-Music3 (AI-generated, instrumental, run locally)` / `MiniMax-Music3 (generada con IA, instrumental, en local)`, printed after "made with"), **and** a sentence in the disclosure paragraph (which stays above "Show more"): EN `The music bed is AI-generated too (MiniMax-Music3, run locally). RapidR is not affiliated with or endorsed by MiniMax.` / ES `La música también está generada con IA (MiniMax-Music3, en local). RapidR no está afiliado ni respaldado por MiniMax.`
   - The metadata `comment` tag carries the disclosure (as in M1 of the demo-mode review) with the music sentence.
   - Pages that embed or link the video (rapidr.dev, GitHub release page, Discussions post, X post text where room allows, groups.io message): the same credit in the caption or first lines next to the video, because that page is the nearest thing to the "user interface".
3. **Instrumental only, no lyrics, no names.** The MLX card says lyrics are required: give section tags only (such as `[intro]`, `[instrumental]`, `[outro]`) or neutral non-sensical text, never words that could be a real song's lyrics. No artist, band, song, film, game, brand or "in the style of" in prompts (Exhibit A items 1 and 12; a sound-alike claim is the main practical risk). Calm, neutral styles.
4. **Provenance log (text only, no audio in git).** For each track used, a line in `docs/video-pipeline.md` or `docs/legal/`: date, model repo and revision (`8ca48eb3...`), MLX-Serve version, full prompt and tags, seed, `duration_seconds`, which candidate was chosen, the video(s) it appears in, and the WAV's SHA-256. Audio stays in git-ignored `dist/` as the other media do.
5. **Never claim exclusivity or ownership of the music.** Do not register it with Content ID or a distributor, do not upload it as its own track anywhere, do not sell or license it, do not offer the audio for download. If a video is published under Creative Commons, state that the music is excluded.
6. **Do not mix inputs.** Never use Apple Loops or any third-party audio as input or reference to the generator (the Apple licence forbids it; this build is text-to-music only anyway). Mixing finished tracks together in the DAW is fine, and each source then gets its own credit.
7. **Pre-publication check.** Listen to the whole track once for a familiar tune. Upload the video as private or unlisted to YouTube first and look at the copyright check result (from memory that YouTube Studio runs one at upload; not re-verified today) before announcing. If a claim or flag appears, do not dispute: swap the track for an Apple Loops bed and re-upload.
8. **Scope.** Videos only. Not in RapidR, Studio, examples, Help, installers, or any asset users receive. Any such use needs a new review.
9. **Restricted-party check once.** Search OFAC's SDN list and the BIS Entity List for MiniMax on the day of the first publication and note the result in this file.
10. **Stop rule.** Re-review if: the LICENSE hash changes, a claim or takedown arrives, a court decision or MiniMax statement touches Music3's training data, or Ruta's revenue from the affected products approaches the $20 million threshold.

## 7. Could not verify

- Whether a **free** video is a "commercial product or service" under clause 3.1 (the licence does not define it), and what "prominently" means for a video; the conditions above are a conservative guess, not an authoritative reading.
- Whether outputs are *licensed to the user* in any sense: the licence is silent and there is no MiniMax FAQ or statement on outputs (nothing found; model@minimax.io was not asked). Asking is optional and free, but an email reply would not be a guarantee.
- The **training data** of Music3 (not documented anywhere I could read), and whether MiniMax faces any music-specific claim (none found; press coverage found concerned video, and the Disney v. MiniMax status comes from a secondary source, https://letsdatascience.com/news/minimax-loses-bid-to-dismiss-disney-copyright-suit-755f915a, dated 27 May 2026).
- The **legal entity** behind "MiniMax" in the licence (not named), governing law (none), and any restricted-party listing (only secondary web search; no hit; the primary OFAC/BIS lists were not queried).
- **YouTube's current wording** on altered/synthetic content for AI music and its upload-time copyright check (only 2023 coverage found); the X, GitHub and groups.io rules were not looked up.
- **EU and Uruguayan** treatment of AI-generated music's copyrightability, and the EU AI Act Art. 50 dates after any amendments (as in the demo-mode review).
- Apple's licence **governing-law and territory** clauses were not read; only section 2.C (Sample Content) and the support article. Whether the Logic Pro in use is a one-time licence or a Paid Subscription (the agreement covers both; if a subscription ends, access to Sample Content ends, but soundtracks already made remain yours to publish: this is my reading of 2.C, not an Apple statement).
- Whether the exact bytes of the local MLX weights Robert has downloaded match the repo revision above (hash the downloaded files and compare with the Hugging Face file list if this matters).

## 8. Sources

- MiniMax-Music3 LICENSE (upstream and MLX build): https://huggingface.co/MiniMaxAI/MiniMax-Music3/raw/main/LICENSE and https://huggingface.co/ddalcu/MiniMax-Music3-MLX-Serve-8bit/raw/main/LICENSE
- Model cards: https://huggingface.co/MiniMaxAI/MiniMax-Music3 , https://huggingface.co/ddalcu/MiniMax-Music3-MLX-Serve-8bit , https://github.com/MiniMax-AI/MiniMax-Music3
- Hugging Face metadata (revisions, tags): https://huggingface.co/api/models/MiniMaxAI/MiniMax-Music3 , https://huggingface.co/api/models/ddalcu/MiniMax-Music3-MLX-Serve-8bit
- Discussions: https://huggingface.co/MiniMaxAI/MiniMax-Music3/discussions/26 , https://huggingface.co/MiniMaxAI/MiniMax-Music3/discussions/27
- MLX-Serve: https://github.com/ddalcu/mlx-serve (MIT)
- Apple Logic Pro licence: installed copy (`/Applications/Logic Pro.app/Contents/Resources/Logic Pro License Agreement.pdf`) and https://support.apple.com/en-us/101908
- US Copyright Office, Copyright and AI Part 2: https://www.copyright.gov/ai/Copyright-and-Artificial-Intelligence-Part-2-Copyrightability-Report.pdf
- YouTube disclosure (2023 coverage): https://www.loeb.com/en/insights/passle/2023/11/youtube-to-crack-down-on-deepfake-music-and-add-generative-ai-disclosure-requirements
- Disney v. MiniMax (secondary): https://letsdatascience.com/news/minimax-loses-bid-to-dismiss-disney-copyright-suit-755f915a
- AI-music litigation background (secondary, none names MiniMax Music3): https://www.technologyreview.com/2024/06/27/1094379/ai-music-suno-udio-lawsuit-record-labels-youtube-licensing/ , https://musosoup.com/blog/google-tries-to-dismiss-indie-artists-lyria-3-ai-training-lawsuit
- Related RapidR reviews: `docs/legal/review-2026-10-30-demo-mode.md` (narration voice, YouTube answers, credits M1/M2/S5/S8), `docs/video-pipeline.md`.
