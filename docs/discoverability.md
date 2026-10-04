# Repository discoverability

CadisWave targets people who need Elgato Wave XLR software for Linux.
The public repository is the main project page.
English and Indonesian documents describe the same supported behavior.

## Search intent and useful answers

| Search intent | Main answer |
| --- | --- |
| Wave XLR software for Linux | [README](../README.md): product summary, installation, screenshots, and support |
| Elgato Wave XLR Linux setup | [Setup guide](wave-xlr-linux.md): USB identity, permissions, controls, and routing |
| Wave Link alternative for Linux | [README FAQ](../README.md#is-cadiswave-a-complete-wave-link-replacement): supported workflows and limits |
| PipeWire microphone and application mixing | [Routing walkthrough](wave-xlr-linux.md#mix-application-and-microphone-audio) |
| Software Wave XLR untuk Linux | [Indonesian README](../README.id.md) and [setup guide](wave-xlr-linux.id.md) |

Keep product facts near the start of each document.
Use clear questions with direct answers.
Link claims to source profiles, test evidence, or official vendor documentation.
Keep the selected icon and actual application screenshots visible.
Retain screenshot provenance and hardware test limits.

## GitHub metadata

The About description starts with the primary product and platform terms:

> Wave XLR software for Linux: Elgato microphone controls and a PipeWire audio mixer. Open-source Rust/GTK4 app by Rama Aditya (CADIS).

The About website points to the [English setup guide](https://github.com/RamaAditya49/cadiswave/blob/main/docs/wave-xlr-linux.md).

Relevant topics:

```text
wave-xlr, elgato, elgato-wave-xlr, linux, linux-audio, pipewire,
audio-mixer, audio-interface, microphone, wave-link, wave-link-alternative,
rust, gtk4, libadwaita, alsa, streaming, open-source
```

GitHub [searches names, descriptions, and topics by default](https://docs.github.com/en/search-github/searching-on-github/searching-for-repositories).
The `in:readme` qualifier includes README content.
[Topics](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/classifying-your-repository-with-topics) connect the repository to related projects and searches.

## Accuracy and maintenance

Verify device controls against [`PROFILES`](../crates/cadiswave-core/src/profiles.rs).
Update both languages when supported behavior changes.
Update [hardware support](hardware-support.md) and [verification](verification.md) when evidence changes.
Record physical checks separately from protocol and device-free tests.
Do not describe old OpenWave releases as current native CadisWave downloads.
Do not claim untested distributions, unavailable controls, guaranteed latency, or search positions.

The README and setup guide contain visible text that search engines and answer systems can use.
Their clear structure supports retrieval; it does not prove inclusion in any answer system.
Google's [AI search guidance](https://developers.google.com/search/docs/appearance/ai-features) applies normal SEO principles to AI Overviews and AI Mode.
Google does not require special AI text files or schema for those features.
Indexing and serving remain subject to Google's systems.
Other answer systems can use different retrieval rules.

GitHub controls the site's page metadata, crawler policy, and domain verification.
Repository files cannot set GitHub's HTML head or domain-wide search settings.
Keep improvements on surfaces the repository owner controls.

## Check discovery after publication

Check current repository metadata:

```bash
gh repo view RamaAditya49/cadiswave \
  --json description,homepageUrl,repositoryTopics
```

Check GitHub repository search:

```bash
gh search repos '"Wave XLR" Linux user:RamaAditya49' \
  --json fullName,url

gh search repos 'topic:wave-xlr user:RamaAditya49' \
  --json fullName,url

gh search repos '"Wave XLR" Linux in:readme user:RamaAditya49' \
  --json fullName,url
```

Search indexes can update after the repository content changes.
A result in these searches does not establish an unscoped position or a Google ranking.

Review GitHub Traffic for available views, clones, and referring sites.
GitHub [retains traffic data for the previous 14 days](https://docs.github.com/en/repositories/viewing-activity-and-data-for-your-repository/viewing-traffic-to-a-repository).
Record aggregate observations outside public source control.
Traffic changes alone do not establish a cause or prove AI citations.

Check public search results for the primary query periodically.
Record the date, query, result URL, and search context when comparing observations.
Do not infer stable ranking from one personalized search.
