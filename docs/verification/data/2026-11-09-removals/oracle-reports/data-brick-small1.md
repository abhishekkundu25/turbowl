| Case | Configuration | Covers | Nemo | OWL-RL | ours | Checks |
|---|---|---|---|---|---|---|
| brick-small1 | default | Brick + small1 | 26381 · - | skipped | 26381 · - | 3 pass, 0 fail, 0 known, 2 not run |
| brick-small1 | complete | Brick + small1 | 47718 · - | skipped | 47718 · - | 3 pass, 0 fail, 0 known, 2 not run |

Cells: closure triples · findings. Nemo and ours run the rules of the row's configuration (default: turbowl::Switches::default(); complete: every switch on); OWL-RL the full W3C rule set, once per input. Fn: an oracle fault (crates/oracle/src/differences.rs) filtered or known on this input.

Not run:
- skipped: OWL-RL: not run on this input (see Options)

Input: 14817 triples from example_models/ontologies/Brick.n3 + example_models/small1.n3. The oracles took 13.8 s.
