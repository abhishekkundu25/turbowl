| Case | Configuration | Covers | Nemo | OWL-RL | ours | Checks |
|---|---|---|---|---|---|---|
| empty | default | cls-thing, cls-nothing1 | 2 · - | 106 · - | 2 · - | 6 pass, 0 fail, 0 known, 2 not run |
| empty | complete | cls-thing, cls-nothing1 | 98 · - | 106 · - | 98 · - | 8 pass, 0 fail, 0 known, 0 not run |
| no-thing-typing | default | cls-thing | 3 · - | 110 · - | 3 · - | 6 pass, 0 fail, 0 known, 2 not run |
| no-thing-typing | complete | cls-thing | 102 · - | 110 · - | 102 · - | 8 pass, 0 fail, 0 known, 0 not run |
| prp-dom | default | prp-dom | 5 · - | 114 · - | 5 · - | 6 pass, 0 fail, 0 known, 2 not run |
| prp-dom | complete | prp-dom | 106 · - | 114 · - | 106 · - | 8 pass, 0 fail, 0 known, 0 not run |
| prp-rng | default | prp-rng, dt-not-type | 6 · - | 115 · - | 6 · - | 6 pass, 0 fail, 0 known, 2 not run |
| prp-rng | complete | prp-rng, dt-not-type | 107 · - | 115 · - | 107 · - | 8 pass, 0 fail, 0 known, 0 not run |
| prp-fp | default | prp-fp, eq-sym, eq-trans | 12 · - | 119 · - | 12 · - | 6 pass, 0 fail, 0 known, 2 not run |
| prp-fp | complete | prp-fp, eq-sym, eq-trans | 129 · - | 119 · - | 129 · - | 8 pass, 0 fail, 0 known, 0 not run; F15 |
| prp-ifp | default | prp-ifp | 9 · - | 116 · - | 9 · - | 6 pass, 0 fail, 0 known, 2 not run |
| prp-ifp | complete | prp-ifp | 108 · - | 116 · - | 108 · - | 8 pass, 0 fail, 0 known, 0 not run |
| prp-ifp-different-objects | default | prp-ifp | 7 · - | 115 · - | 7 · - | 6 pass, 0 fail, 0 known, 2 not run |
| prp-ifp-different-objects | complete | prp-ifp | 107 · - | 115 · - | 107 · - | 8 pass, 0 fail, 0 known, 0 not run |
| prp-symp | default | prp-symp | 5 · - | 113 · - | 5 · - | 6 pass, 0 fail, 0 known, 2 not run |
| prp-symp | complete | prp-symp | 105 · - | 113 · - | 105 · - | 8 pass, 0 fail, 0 known, 0 not run |
| prp-trp | default | prp-trp | 9 · - | 119 · - | 9 · - | 6 pass, 0 fail, 0 known, 2 not run |
| prp-trp | complete | prp-trp | 111 · - | 119 · - | 111 · - | 8 pass, 0 fail, 0 known, 0 not run |
| prp-spo1 | default | prp-spo1 | 7 · - | 118 · - | 7 · - | 6 pass, 0 fail, 0 known, 2 not run |
| prp-spo1 | complete | prp-spo1 | 110 · - | 118 · - | 110 · - | 8 pass, 0 fail, 0 known, 0 not run |
| prp-eqp | default | prp-eqp1, prp-eqp2 | 7 · - | 122 · - | 7 · - | 6 pass, 0 fail, 0 known, 2 not run |
| prp-eqp | complete | prp-eqp1, prp-eqp2 | 118 · - | 122 · - | 118 · - | 8 pass, 0 fail, 0 known, 0 not run; F1 |
| prp-inv | default | prp-inv1, prp-inv2 | 7 · - | 118 · - | 7 · - | 6 pass, 0 fail, 0 known, 2 not run |
| prp-inv | complete | prp-inv1, prp-inv2 | 110 · - | 118 · - | 110 · - | 8 pass, 0 fail, 0 known, 0 not run |
| cls-int | default | cls-int1, cls-int2 | 14 · - | 132 · - | 14 · - | 6 pass, 0 fail, 0 known, 2 not run |
| cls-int | complete | cls-int1, cls-int2 | 124 · - | 132 · - | 124 · - | 8 pass, 0 fail, 0 known, 0 not run |
| two-intersections | default | cls-int1, cls-int2 | 24 · - | 148 · - | 24 · - | 6 pass, 0 fail, 0 known, 2 not run |
| two-intersections | complete | cls-int1, cls-int2 | 140 · - | 148 · - | 140 · - | 8 pass, 0 fail, 0 known, 0 not run |
| shared-list-tail | default | cls-int1, lists | 10 · - | 128 · - | 10 · - | 6 pass, 0 fail, 0 known, 2 not run |
| shared-list-tail | complete | cls-int1, lists | 120 · - | 128 · - | 120 · - | 8 pass, 0 fail, 0 known, 0 not run |
| list-cell-without-first | default | cls-int1, cls-int2, cls-uni, lists | 9 · - | 132 · - | 9 · - | 6 pass, 0 fail, 0 known, 2 not run |
| list-cell-without-first | complete | cls-int1, cls-int2, cls-uni, lists | 117 · - | 132 · - | 117 · - | 8 pass, 0 fail, 0 known, 0 not run |
| list-rest-stops-at-nil | default | cls-int1, cls-int2, cls-uni, lists | 15 · - | failed | 15 · - | 3 pass, 0 fail, 3 known, 2 not run; F4 |
| list-rest-stops-at-nil | complete | cls-int1, cls-int2, cls-uni, lists | 127 · - | failed | 127 · - | 5 pass, 0 fail, 3 known, 0 not run; F4 |
| list-nil-head | default | cls-uni, lists | 12 · - | failed | 12 · - | 3 pass, 0 fail, 3 known, 2 not run; F4 |
| list-nil-head | complete | cls-uni, lists | 121 · - | failed | 121 · - | 5 pass, 0 fail, 3 known, 0 not run; F4 |
| derived-list | default | cls-int1, cls-uni, lists | 16 · - | 141 · - | 16 · - | 6 pass, 0 fail, 0 known, 2 not run |
| derived-list | complete | cls-int1, cls-uni, lists | 137 · - | 141 · - | 137 · - | 8 pass, 0 fail, 0 known, 0 not run; F1 |
| cls-uni | default | cls-uni | 11 · - | 128 · - | 11 · - | 6 pass, 0 fail, 0 known, 2 not run |
| cls-uni | complete | cls-uni | 120 · - | 128 · - | 120 · - | 8 pass, 0 fail, 0 known, 0 not run |
| cls-oo | default | cls-oo | 9 · - | 122 · - | 9 · - | 6 pass, 0 fail, 0 known, 2 not run |
| cls-oo | complete | cls-oo | 114 · - | 122 · - | 114 · - | 8 pass, 0 fail, 0 known, 0 not run |
| cls-oo-cell-without-first | default | cls-oo, lists | 6 · - | 119 · - | 6 · - | 6 pass, 0 fail, 0 known, 2 not run |
| cls-oo-cell-without-first | complete | cls-oo, lists | 110 · - | 119 · - | 110 · - | 8 pass, 0 fail, 0 known, 0 not run |
| cls-svf | default | cls-svf1, cls-svf2 | 12 · - | 131 · - | 12 · - | 6 pass, 0 fail, 0 known, 2 not run |
| cls-svf | complete | cls-svf1, cls-svf2 | 123 · - | 131 · - | 123 · - | 8 pass, 0 fail, 0 known, 0 not run |
| cls-avf | default | cls-avf | 8 · - | 121 · - | 8 · - | 6 pass, 0 fail, 0 known, 2 not run |
| cls-avf | complete | cls-avf | 113 · - | 121 · - | 113 · - | 8 pass, 0 fail, 0 known, 0 not run |
| cls-hv | default | cls-hv1, cls-hv2 | 8 · - | 119 · - | 8 · - | 6 pass, 0 fail, 0 known, 2 not run |
| cls-hv | complete | cls-hv1, cls-hv2 | 111 · - | 119 · - | 111 · - | 8 pass, 0 fail, 0 known, 0 not run |
| cax-sco-scm-sco | default | cax-sco, scm-sco | 8 · - | 116 · - | 8 · - | 6 pass, 0 fail, 0 known, 2 not run |
| cax-sco-scm-sco | complete | cax-sco, scm-sco | 108 · - | 116 · - | 108 · - | 8 pass, 0 fail, 0 known, 0 not run |
| cax-eqc-scm-eqc1 | default | cax-eqc1, cax-eqc2, scm-eqc1 | 11 · - | 118 · - | 11 · - | 6 pass, 0 fail, 0 known, 2 not run; F1 |
| cax-eqc-scm-eqc1 | complete | cax-eqc1, cax-eqc2, scm-eqc1 | 114 · - | 118 · - | 114 · - | 8 pass, 0 fail, 0 known, 0 not run; F1 |
| no-container-axioms | default | not rdfs12 | 3 · - | 110 · - | 3 · - | 6 pass, 0 fail, 0 known, 2 not run |
| no-container-axioms | complete | not rdfs12 | 102 · - | 110 · - | 102 · - | 8 pass, 0 fail, 0 known, 0 not run |
| language-tag | default | prp-spo1 | 5 · - | 112 · - | 5 · - | 6 pass, 0 fail, 0 known, 2 not run |
| language-tag | complete | prp-spo1 | 104 · - | 112 · - | 104 · - | 8 pass, 0 fail, 0 known, 0 not run |
| eq-sym-trans | default | eq-sym, eq-trans | 11 · - | 115 · - | 11 · - | 6 pass, 0 fail, 0 known, 2 not run |
| eq-sym-trans | complete | eq-sym, eq-trans | 107 · - | 115 · - | 107 · - | 8 pass, 0 fail, 0 known, 0 not run |
| eq-rep-s-p-o | default | eq-rep-s, eq-rep-p, eq-rep-o | 22 · - | 126 · - | 22 · - | 6 pass, 0 fail, 0 known, 2 not run |
| eq-rep-s-p-o | complete | eq-rep-s, eq-rep-p, eq-rep-o | 118 · - | 126 · - | 118 · - | 8 pass, 0 fail, 0 known, 0 not run |
| eq-vocabulary-term | default | eq-rep-p, eq-rep-s, eq-rep-o, cax-sco | 13 · - | 164 · - | 13 · - | 6 pass, 0 fail, 0 known, 2 not run |
| eq-vocabulary-term | complete | eq-rep-p, eq-rep-s, eq-rep-o, cax-sco | 152 · - | 164 · - | 152 · - | 6 pass, 0 fail, 0 known, 2 not run |
| eq-two-vocabulary-terms | default | eq-rep-p, cax-eqc2, scm-eqc1 | 16 · - | 120 · - | 16 · - | 6 pass, 0 fail, 0 known, 2 not run; F1 |
| eq-two-vocabulary-terms | complete | eq-rep-p, cax-eqc2, scm-eqc1 | 116 · - | 120 · - | 116 · - | 6 pass, 0 fail, 0 known, 2 not run; F1 |
| eq-same-as-a-property | default | eq-rep-p, eq-rep-s, eq-sym | 20 · - | 180 · - | 20 · - | 6 pass, 0 fail, 0 known, 2 not run |
| eq-same-as-a-property | complete | eq-rep-p, eq-rep-s, eq-sym | 168 · - | 180 · - | 168 · - | 6 pass, 0 fail, 0 known, 2 not run |
| eq-thing | default | eq-rep-o, cls-svf2 | 12 · - | 130 · - | 12 · - | 6 pass, 0 fail, 0 known, 2 not run |
| eq-thing | complete | eq-rep-o, cls-svf2 | 122 · - | 130 · - | 122 · - | 6 pass, 0 fail, 0 known, 2 not run |
| eq-nil-cell | default | eq-rep-o, cls-int2, prp-spo2, lists | 24 · - | failed | 24 · - | 3 pass, 0 fail, 3 known, 2 not run; F4 |
| eq-nil-cell | complete | eq-rep-o, cls-int2, prp-spo2, lists | 135 · - | failed | 135 · - | 3 pass, 0 fail, 3 known, 2 not run; F4 |
| eq-literal-typed | default | eq-rep-s, dt-not-type | 8 · - | 115 · - | 8 · dt-not-type | 5 pass, 0 fail, 1 known, 2 not run; F5 |
| eq-literal-typed | complete | eq-rep-s, dt-not-type | 116 · - | 115 · - | 116 · dt-not-type | 7 pass, 0 fail, 1 known, 0 not run; F5, F15 |
| eq-findings | default | eq-rep-s, eq-rep-o, prp-irp, cax-dw | 20 · cax-dw×2 prp-irp×2 | 129 · cax-dw×2 prp-irp×2 | 20 · cax-dw×2 prp-irp×2 | 6 pass, 0 fail, 0 known, 2 not run |
| eq-findings | complete | eq-rep-s, eq-rep-o, prp-irp, cax-dw | 121 · cax-dw×2 prp-irp×2 | 129 · cax-dw×2 prp-irp×2 | 121 · cax-dw×2 prp-irp×2 | 8 pass, 0 fail, 0 known, 0 not run |
| prp-spo2 | default | prp-spo2 | 18 · - | 140 · - | 18 · - | 6 pass, 0 fail, 0 known, 2 not run |
| prp-spo2 | complete | prp-spo2 | 132 · - | 140 · - | 132 · - | 8 pass, 0 fail, 0 known, 0 not run |
| chain-no-transitivity | default | prp-spo2 | 11 · - | 127 · - | 11 · - | 6 pass, 0 fail, 0 known, 2 not run |
| chain-no-transitivity | complete | prp-spo2 | 119 · - | 127 · - | 119 · - | 8 pass, 0 fail, 0 known, 0 not run |
| chain-cyclic-list | default | prp-spo2 | 9 · - | failed | 9 · - | 3 pass, 0 fail, 3 known, 2 not run; F4 |
| chain-cyclic-list | complete | prp-spo2 | 116 · - | failed | 116 · - | 5 pass, 0 fail, 3 known, 0 not run; F4 |
| chain-nil-head | default | prp-spo2, lists | 12 · - | failed | 12 · - | 3 pass, 0 fail, 3 known, 2 not run; F4 |
| chain-nil-head | complete | prp-spo2, lists | 120 · - | failed | 120 · - | 5 pass, 0 fail, 3 known, 0 not run; F4 |
| cax-dw | default | cax-dw | 5 · cax-dw | 113 · cax-dw | 5 · cax-dw | 6 pass, 0 fail, 0 known, 2 not run |
| cax-dw | complete | cax-dw | 105 · cax-dw | 113 · cax-dw | 105 · cax-dw | 8 pass, 0 fail, 0 known, 0 not run |
| prp-pdw | default | prp-pdw | 5 · prp-pdw | 114 · prp-pdw | 5 · prp-pdw | 6 pass, 0 fail, 0 known, 2 not run |
| prp-pdw | complete | prp-pdw | 106 · prp-pdw | 114 · prp-pdw | 106 · prp-pdw | 8 pass, 0 fail, 0 known, 0 not run |
| prp-asyp | default | prp-asyp | 5 · prp-asyp×2 | 113 · prp-asyp×2 | 5 · prp-asyp×2 | 6 pass, 0 fail, 0 known, 2 not run |
| prp-asyp | complete | prp-asyp | 105 · prp-asyp×2 | 113 · prp-asyp×2 | 105 · prp-asyp×2 | 8 pass, 0 fail, 0 known, 0 not run |
| prp-irp | default | prp-irp | 4 · prp-irp | 111 · prp-irp | 4 · prp-irp | 6 pass, 0 fail, 0 known, 2 not run |
| prp-irp | complete | prp-irp | 103 · prp-irp | 111 · prp-irp | 103 · prp-irp | 8 pass, 0 fail, 0 known, 0 not run |
| cls-nothing2 | default | cls-nothing2 | 3 · cls-nothing2 | 109 · cls-nothing2 | 3 · cls-nothing2 | 6 pass, 0 fail, 0 known, 2 not run |
| cls-nothing2 | complete | cls-nothing2 | 101 · cls-nothing2 | 109 · cls-nothing2 | 101 · cls-nothing2 | 8 pass, 0 fail, 0 known, 0 not run |
| cls-com | default | cls-com | 5 · cls-com | 113 · cls-com | 5 · cls-com | 5 pass, 0 fail, 1 known, 2 not run; F2 |
| cls-com | complete | cls-com | 105 · cls-com | 113 · cls-com | 105 · cls-com | 7 pass, 0 fail, 1 known, 0 not run; F2 |
| eq-diff1 | default | eq-diff1 | 10 · eq-diff1×4 | 115 · eq-diff1×4 | 10 · eq-diff1×4 | 6 pass, 0 fail, 0 known, 2 not run |
| eq-diff1 | complete | eq-diff1 | 107 · eq-diff1×4 | 115 · eq-diff1×4 | 107 · eq-diff1×4 | 8 pass, 0 fail, 0 known, 0 not run |
| eq-diff2 | default | eq-diff2 | 16 · eq-diff2×4 | 130 · eq-diff2 | 16 · eq-diff2×4 | 5 pass, 0 fail, 1 known, 2 not run; F6 |
| eq-diff2 | complete | eq-diff2 | 122 · eq-diff2×4 | 130 · eq-diff2 | 122 · eq-diff2×4 | 7 pass, 0 fail, 1 known, 0 not run; F6 |
| eq-diff3 | default | eq-diff3 | 16 · eq-diff3×4 | 130 · eq-diff3 | 16 · eq-diff3×4 | 5 pass, 0 fail, 1 known, 2 not run; F6 |
| eq-diff3 | complete | eq-diff3 | 122 · eq-diff3×4 | 130 · eq-diff3 | 122 · eq-diff3×4 | 7 pass, 0 fail, 1 known, 0 not run; F6 |
| cax-adc | default | cax-adc | 13 · cax-adc | 131 · cax-adc | 13 · cax-adc | 6 pass, 0 fail, 0 known, 2 not run |
| cax-adc | complete | cax-adc | 123 · cax-adc | 131 · cax-adc | 123 · cax-adc | 8 pass, 0 fail, 0 known, 0 not run |
| prp-adp | default | prp-adp | 13 · prp-adp | 132 · prp-adp | 13 · prp-adp | 6 pass, 0 fail, 0 known, 2 not run |
| prp-adp | complete | prp-adp | 124 · prp-adp | 132 · prp-adp | 124 · prp-adp | 8 pass, 0 fail, 0 known, 0 not run |
| prp-npa1 | default | prp-npa1 | 8 · prp-npa1 | 121 · prp-npa1 | 8 · prp-npa1 | 6 pass, 0 fail, 0 known, 2 not run |
| prp-npa1 | complete | prp-npa1 | 113 · prp-npa1 | 121 · prp-npa1 | 113 · prp-npa1 | 8 pass, 0 fail, 0 known, 0 not run |
| prp-npa2 | default | prp-npa2 | 8 · prp-npa2 | 119 · prp-npa2 | 8 · prp-npa2 | 6 pass, 0 fail, 0 known, 2 not run |
| prp-npa2 | complete | prp-npa2 | 111 · prp-npa2 | 119 · prp-npa2 | 111 · prp-npa2 | 8 pass, 0 fail, 0 known, 0 not run |
| cls-maxc1 | default | cls-maxc1 | 8 · cls-maxc1 | 120 · cls-maxc1 | 8 · cls-maxc1 | 6 pass, 0 fail, 0 known, 2 not run |
| cls-maxc1 | complete | cls-maxc1 | 112 · cls-maxc1 | 120 · cls-maxc1 | 112 · cls-maxc1 | 8 pass, 0 fail, 0 known, 0 not run |
| cls-maxqc1 | default | cls-maxqc1 | 9 · cls-maxqc1 | 122 · cls-maxqc1 | 9 · cls-maxqc1 | 6 pass, 0 fail, 0 known, 2 not run |
| cls-maxqc1 | complete | cls-maxqc1 | 114 · cls-maxqc1 | 122 · cls-maxqc1 | 114 · cls-maxqc1 | 8 pass, 0 fail, 0 known, 0 not run |
| cls-maxqc2 | default | cls-maxqc2 | 7 · cls-maxqc2 | 118 · cls-maxqc2 | 7 · cls-maxqc2 | 6 pass, 0 fail, 0 known, 2 not run |
| cls-maxqc2 | complete | cls-maxqc2 | 110 · cls-maxqc2 | 118 · cls-maxqc2 | 110 · cls-maxqc2 | 8 pass, 0 fail, 0 known, 0 not run |
| dt-not-type | default | dt-not-type | 8 · - | 114 · dt-not-type×2 | 8 · dt-not-type×3 | 5 pass, 0 fail, 1 known, 2 not run; F3 |
| dt-not-type | complete | dt-not-type | 106 · - | 114 · dt-not-type×2 | 106 · dt-not-type×3 | 7 pass, 0 fail, 1 known, 0 not run; F3 |
| dt-not-type-map | default | dt-not-type | 43 · - | 195 · dt-not-type×19 | 43 · dt-not-type×19 | 6 pass, 0 fail, 0 known, 2 not run |
| dt-not-type-map | complete | dt-not-type | 143 · - | 195 · dt-not-type×19 | 143 · dt-not-type×19 | 8 pass, 0 fail, 0 known, 0 not run; F15 |
| dt-not-type-range | default | prp-rng, dt-not-type | 16 · - | 128 · - | 16 · dt-not-type×3 | 5 pass, 0 fail, 1 known, 2 not run; F5 |
| dt-not-type-range | complete | prp-rng, dt-not-type | 122 · - | 128 · - | 122 · dt-not-type×3 | 7 pass, 0 fail, 1 known, 0 not run; F5, F15 |
| dt-not-type-owl-2-rl | default | dt-not-type, prp-rng | 9 · - | 118 · - | 9 · dt-not-type | 5 pass, 0 fail, 1 known, 2 not run; F5 |
| dt-not-type-owl-2-rl | complete | dt-not-type, prp-rng | 111 · - | 118 · - | 111 · dt-not-type | 7 pass, 0 fail, 1 known, 0 not run; F5 |
| prp-key | default | prp-key | 12 · - | 136 · - | 12 · - | 3 pass, 0 fail, 0 known, 5 not run; F13 |
| prp-key | complete | prp-key | 121 · - | 136 · - | 121 · - | 8 pass, 0 fail, 0 known, 0 not run; F13 |
| prp-key-two-properties | default | prp-key | 22 · - | 174 · - | 22 · - | 3 pass, 0 fail, 0 known, 5 not run; F13 |
| prp-key-two-properties | complete | prp-key | 144 · - | 174 · - | 144 · - | 8 pass, 0 fail, 0 known, 0 not run; F13 |
| prp-key-equal-values | default | prp-key, eq-rep-o | 15 · - | 130 · - | 15 · - | 3 pass, 0 fail, 0 known, 5 not run; F13 |
| prp-key-equal-values | complete | prp-key, eq-rep-o | 122 · - | 130 · - | 122 · - | 8 pass, 0 fail, 0 known, 0 not run |
| cls-maxc2 | default | cls-maxc2 | 9 · - | 125 · - | 9 · - | 3 pass, 0 fail, 0 known, 5 not run |
| cls-maxc2 | complete | cls-maxc2 | 117 · - | 125 · - | 117 · - | 8 pass, 0 fail, 0 known, 0 not run |
| cls-maxc2-other-numbers | default | cls-maxc2 | 10 · - | 126 · - | 10 · - | 3 pass, 0 fail, 0 known, 5 not run; F14 |
| cls-maxc2-other-numbers | complete | cls-maxc2 | 114 · - | 126 · - | 114 · - | 8 pass, 0 fail, 0 known, 0 not run; F14 |
| cls-maxqc3 | default | cls-maxqc3 | 11 · - | 127 · - | 11 · - | 3 pass, 0 fail, 0 known, 5 not run |
| cls-maxqc3 | complete | cls-maxqc3 | 119 · - | 127 · - | 119 · - | 8 pass, 0 fail, 0 known, 0 not run |
| cls-maxqc4 | default | cls-maxqc4 | 8 · - | 122 · - | 8 · - | 3 pass, 0 fail, 0 known, 5 not run |
| cls-maxqc4 | complete | cls-maxqc4 | 114 · - | 122 · - | 114 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-sco-cycle | default | scm-sco, scm-eqc2 | 6 · - | 112 · - | 6 · - | 3 pass, 0 fail, 0 known, 5 not run; F1 |
| scm-sco-cycle | complete | scm-sco, scm-eqc2 | 108 · - | 112 · - | 108 · - | 8 pass, 0 fail, 0 known, 0 not run; F1 |
| scm-eqc2-one-way | default | scm-eqc2 | 5 · - | 112 · - | 5 · - | 3 pass, 0 fail, 0 known, 5 not run |
| scm-eqc2-one-way | complete | scm-eqc2 | 104 · - | 112 · - | 104 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-spo | default | scm-spo, prp-spo1 | 10 · - | 129 · - | 10 · - | 3 pass, 0 fail, 0 known, 5 not run |
| scm-spo | complete | scm-spo, prp-spo1 | 121 · - | 129 · - | 121 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-eqp1 | default | scm-eqp1, scm-spo, scm-eqp2 | 5 · - | 121 · - | 5 · - | 3 pass, 0 fail, 0 known, 5 not run |
| scm-eqp1 | complete | scm-eqp1, scm-spo, scm-eqp2 | 113 · - | 121 · - | 113 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-eqp2 | default | scm-eqp2 | 8 · - | 127 · - | 8 · - | 3 pass, 0 fail, 0 known, 5 not run |
| scm-eqp2 | complete | scm-eqp2 | 119 · - | 127 · - | 119 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-dom1 | default | scm-dom1, prp-dom | 7 · - | 118 · - | 7 · - | 3 pass, 0 fail, 0 known, 5 not run |
| scm-dom1 | complete | scm-dom1, prp-dom | 110 · - | 118 · - | 110 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-dom2 | default | scm-dom2, prp-dom | 7 · - | 119 · - | 7 · - | 3 pass, 0 fail, 0 known, 5 not run |
| scm-dom2 | complete | scm-dom2, prp-dom | 111 · - | 119 · - | 111 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-rng1 | default | scm-rng1, prp-rng | 7 · - | 118 · - | 7 · - | 3 pass, 0 fail, 0 known, 5 not run |
| scm-rng1 | complete | scm-rng1, prp-rng | 110 · - | 118 · - | 110 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-rng2 | default | scm-rng2, prp-rng | 7 · - | 119 · - | 7 · - | 3 pass, 0 fail, 0 known, 5 not run |
| scm-rng2 | complete | scm-rng2, prp-rng | 111 · - | 119 · - | 111 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-hv | default | scm-hv, scm-eqc1, scm-sco | 34 · - | 184 · - | 34 · - | 5 pass, 0 fail, 0 known, 3 not run |
| scm-hv | complete | scm-hv, scm-eqc1, scm-sco | 176 · - | 184 · - | 176 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-svf1 | default | scm-svf1, scm-eqc1, scm-sco, cax-sco | 26 · - | 175 · - | 26 · - | 5 pass, 0 fail, 0 known, 3 not run |
| scm-svf1 | complete | scm-svf1, scm-eqc1, scm-sco, cax-sco | 167 · - | 175 · - | 167 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-svf1-one-filler | default | scm-svf1 | 6 · - | 116 · - | 6 · - | 3 pass, 0 fail, 0 known, 5 not run |
| scm-svf1-one-filler | complete | scm-svf1 | 108 · - | 116 · - | 108 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-svf2 | default | scm-svf2, scm-eqc1, scm-sco | 24 · - | 167 · - | 24 · - | 5 pass, 0 fail, 0 known, 3 not run |
| scm-svf2 | complete | scm-svf2, scm-eqc1, scm-sco | 159 · - | 167 · - | 159 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-avf1 | default | scm-avf1, scm-eqc1, scm-sco, cls-avf | 29 · - | 180 · - | 29 · - | 5 pass, 0 fail, 0 known, 3 not run |
| scm-avf1 | complete | scm-avf1, scm-eqc1, scm-sco, cls-avf | 172 · - | 180 · - | 172 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-avf2 | default | scm-avf2, scm-eqc1, scm-sco | 24 · - | 167 · - | 24 · - | 5 pass, 0 fail, 0 known, 3 not run |
| scm-avf2 | complete | scm-avf2, scm-eqc1, scm-sco | 159 · - | 167 · - | 159 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-int | default | scm-int | 7 · - | 122 · - | 7 · - | 3 pass, 0 fail, 0 known, 5 not run |
| scm-int | complete | scm-int | 114 · - | 122 · - | 114 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-uni | default | scm-uni | 7 · - | 122 · - | 7 · - | 3 pass, 0 fail, 0 known, 5 not run |
| scm-uni | complete | scm-uni | 114 · - | 122 · - | 114 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-int-ill-formed | default | scm-int | 6 · - | 119 · - | 6 · - | 3 pass, 0 fail, 0 known, 5 not run |
| scm-int-ill-formed | complete | scm-int | 110 · - | 119 · - | 110 · - | 8 pass, 0 fail, 0 known, 0 not run |
| eq-ref | default | eq-ref | 4 · - | 112 · - | 4 · - | 3 pass, 0 fail, 0 known, 5 not run |
| eq-ref | complete | eq-ref | 104 · - | 112 · - | 104 · - | 8 pass, 0 fail, 0 known, 0 not run |
| eq-ref-equal-terms | default | eq-ref, eq-sym, eq-trans | 8 · - | 114 · - | 8 · - | 3 pass, 0 fail, 0 known, 5 not run |
| eq-ref-equal-terms | complete | eq-ref, eq-sym, eq-trans | 106 · - | 114 · - | 106 · - | 8 pass, 0 fail, 0 known, 0 not run |
| eq-ref-read-by-a-rule | default | eq-ref, prp-dom, prp-spo1 | 5 · - | 238 · - | 5 · - | 3 pass, 0 fail, 0 known, 5 not run |
| eq-ref-read-by-a-rule | complete | eq-ref, prp-dom, prp-spo1 | 222 · - | 238 · - | 222 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-cls | default | scm-cls | 4 · - | 115 · - | 4 · - | 3 pass, 0 fail, 0 known, 5 not run |
| scm-cls | complete | scm-cls | 107 · - | 115 · - | 107 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-op | default | scm-op | 4 · - | 117 · - | 4 · - | 3 pass, 0 fail, 0 known, 5 not run |
| scm-op | complete | scm-op | 109 · - | 117 · - | 109 · - | 8 pass, 0 fail, 0 known, 0 not run |
| scm-dp | default | scm-dp | 4 · - | 115 · - | 4 · - | 3 pass, 0 fail, 0 known, 5 not run |
| scm-dp | complete | scm-dp | 108 · - | 115 · - | 108 · - | 8 pass, 0 fail, 0 known, 0 not run; F15 |
| prp-ap | default | prp-ap | 3 · - | 110 · - | 3 · - | 3 pass, 0 fail, 0 known, 5 not run |
| prp-ap | complete | prp-ap | 102 · - | 110 · - | 102 · - | 8 pass, 0 fail, 0 known, 0 not run |
| dt-type1 | default | dt-type1 | 3 · - | 110 · - | 3 · - | 3 pass, 0 fail, 0 known, 5 not run |
| dt-type1 | complete | dt-type1 | 102 · - | 110 · - | 102 · - | 8 pass, 0 fail, 0 known, 0 not run |
| dt-type2 | default | dt-type2, eq-rep-s, cls-svf1 | 10 · - | 128 · - | 10 · - | 3 pass, 0 fail, 0 known, 5 not run |
| dt-type2 | complete | dt-type2, eq-rep-s, cls-svf1 | 126 · - | 128 · - | 126 · - | 8 pass, 0 fail, 0 known, 0 not run; F15 |
| eq-diff1-eq-ref | default | eq-diff1, eq-ref | 4 · - | 112 · eq-diff1 | 4 · - | 3 pass, 0 fail, 0 known, 5 not run |
| eq-diff1-eq-ref | complete | eq-diff1, eq-ref | 104 · eq-diff1 | 112 · eq-diff1 | 104 · eq-diff1 | 8 pass, 0 fail, 0 known, 0 not run |
| eq-diff2-eq-ref | default | eq-diff2, eq-diff3, eq-ref | 22 · - | 145 · - | 22 · - | 3 pass, 0 fail, 0 known, 5 not run |
| eq-diff2-eq-ref | complete | eq-diff2, eq-diff3, eq-ref | 137 · eq-diff2 eq-diff3 | 145 · - | 137 · eq-diff2 eq-diff3 | 7 pass, 0 fail, 1 known, 0 not run; F6 |
| dt-eq | default | dt-eq, eq-rep-o | 7 · - | 117 · - | 7 · - | 3 pass, 0 fail, 0 known, 5 not run |
| dt-eq | complete | dt-eq, eq-rep-o | 112 · - | 117 · - | 112 · - | 8 pass, 0 fail, 0 known, 0 not run; F15 |
| dt-eq-iri | default | dt-eq, eq-sym, eq-trans | 8 · - | 121 · - | 8 · - | 3 pass, 0 fail, 0 known, 5 not run |
| dt-eq-iri | complete | dt-eq, eq-sym, eq-trans | 136 · - | 121 · - | 136 · - | 8 pass, 0 fail, 0 known, 0 not run; F15 |
| dt-eq-cardinality | default | dt-eq, eq-rep-o, cls-maxc2 | 7 · - | 120 · - | 7 · - | 3 pass, 0 fail, 0 known, 5 not run |
| dt-eq-cardinality | complete | dt-eq, eq-rep-o, cls-maxc2 | 113 · - | 120 · - | 113 · - | 8 pass, 0 fail, 0 known, 0 not run; F15 |
| dt-diff | default | dt-diff, eq-diff1, prp-fp | 5 · - | 112 · - | 5 · - | 3 pass, 0 fail, 0 known, 5 not run |
| dt-diff | complete | dt-diff, eq-diff1, prp-fp | 105 · eq-diff1×9 | 112 · - | 105 · eq-diff1×9 | 7 pass, 0 fail, 1 known, 0 not run; F15 |
| dt-diff-never-listed | default | dt-diff | 5 · - | 114 · - | 5 · - | 3 pass, 0 fail, 0 known, 5 not run |
| dt-diff-never-listed | complete | dt-diff | 116 · - | 114 · - | 116 · - | 8 pass, 0 fail, 0 known, 0 not run; F15 |

Cells: closure triples · findings. Nemo and ours run the rules of the row's configuration (default: turbowl::Switches::default(); complete: every switch on); OWL-RL the full W3C rule set, once per input. Fn: an oracle fault (crates/oracle/src/differences.rs) filtered or known on this input.

### list-rest-stops-at-nil (default): OWL-RL: expectations (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:     for cl in self._list(x):
              ~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### list-rest-stops-at-nil (default): Nemo ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:     for cl in self._list(x):
              ~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### list-rest-stops-at-nil (default): ours ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:     for cl in self._list(x):
              ~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### list-rest-stops-at-nil (complete): OWL-RL: expectations (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:     for cl in self._list(x):
              ~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### list-rest-stops-at-nil (complete): Nemo ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:     for cl in self._list(x):
              ~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### list-rest-stops-at-nil (complete): ours ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:     for cl in self._list(x):
              ~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### list-nil-head (default): OWL-RL: expectations (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:     for cl in self._list(x):
              ~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### list-nil-head (default): Nemo ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:     for cl in self._list(x):
              ~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### list-nil-head (default): ours ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:     for cl in self._list(x):
              ~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### list-nil-head (complete): OWL-RL: expectations (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:     for cl in self._list(x):
              ~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### list-nil-head (complete): Nemo ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:     for cl in self._list(x):
              ~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### list-nil-head (complete): ours ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:     for cl in self._list(x):
              ~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### eq-nil-cell (default): OWL-RL: expectations (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 659, in _classes
    classes = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### eq-nil-cell (default): Nemo ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 659, in _classes
    classes = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### eq-nil-cell (default): ours ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 659, in _classes
    classes = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### eq-nil-cell (complete): OWL-RL: expectations (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 659, in _classes
    classes = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### eq-nil-cell (complete): Nemo ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 659, in _classes
    classes = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### eq-nil-cell (complete): ours ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 659, in _classes
    classes = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### eq-literal-typed (default): OWL-RL: expectations (known: F5, owlrl 7.6.2 runs dt-not-type only on literals against their own datatype (OWLRL.py:217-232), never on `lt rdf:type dt` triples, so a literal typed by prp-rng is not checked; the W3C dt-not-type matches any such triple, and the W3C test rdf-mt datatypes-range-clash expects "25"^^xsd:integer in an xsd:string range to be inconsistent)

```
! missing dt-not-type "abc" <http://www.w3.org/2001/XMLSchema#integer>
```

### eq-literal-typed (complete): OWL-RL: expectations (known: F5, owlrl 7.6.2 runs dt-not-type only on literals against their own datatype (OWLRL.py:217-232), never on `lt rdf:type dt` triples, so a literal typed by prp-rng is not checked; the W3C dt-not-type matches any such triple, and the W3C test rdf-mt datatypes-range-clash expects "25"^^xsd:integer in an xsd:string range to be inconsistent)

```
! missing dt-not-type "abc" <http://www.w3.org/2001/XMLSchema#integer>
```

### chain-cyclic-list (default): OWL-RL: expectations (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 344, in _property_chain
    chain = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### chain-cyclic-list (default): Nemo ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 344, in _property_chain
    chain = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### chain-cyclic-list (default): ours ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 344, in _property_chain
    chain = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### chain-cyclic-list (complete): OWL-RL: expectations (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 344, in _property_chain
    chain = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### chain-cyclic-list (complete): Nemo ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 344, in _property_chain
    chain = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### chain-cyclic-list (complete): ours ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 344, in _property_chain
    chain = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### chain-nil-head (default): OWL-RL: expectations (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 344, in _property_chain
    chain = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### chain-nil-head (default): Nemo ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 344, in _property_chain
    chain = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### chain-nil-head (default): ours ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 344, in _property_chain
    chain = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### chain-nil-head (complete): OWL-RL: expectations (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 344, in _property_chain
    chain = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### chain-nil-head (complete): Nemo ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 344, in _property_chain
    chain = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### chain-nil-head (complete): ours ⊆ OWL-RL (known: F4, rdflib's Collection raises "List contains a recursive rdf:rest reference" on a cyclic list, so owlrl 7.6.2 stops with an error; the W3C rules derive nothing from a list that never reaches rdf:nil)

```
OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 344, in _property_chain
    chain = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
```

### cls-com (default): OWL-RL: expectations (known: F2, owlrl 7.6.2 formats cls-com's message with `x`, which is the second class there (OWLRL.py:684-691), so its finding is (c1, c2, c2), not (c1, c2, individual))

```
! missing cls-com <http://example.org/C1> <http://example.org/C2> <http://example.org/x>
! unexpected cls-com <http://example.org/C1> <http://example.org/C2> <http://example.org/C2>
```

### cls-com (complete): OWL-RL: expectations (known: F2, owlrl 7.6.2 formats cls-com's message with `x`, which is the second class there (OWLRL.py:684-691), so its finding is (c1, c2, c2), not (c1, c2, individual))

```
! missing cls-com <http://example.org/C1> <http://example.org/C2> <http://example.org/x>
! unexpected cls-com <http://example.org/C1> <http://example.org/C2> <http://example.org/C2>
```

### eq-diff2 (default): OWL-RL: expectations (known: F6, owlrl 7.6.2 reads the list of eq-diff2, eq-diff3 (OWLRL.py:416-436) and prp-adp (OWLRL.py:517-531) through rdflib's `items`, one rdf:first per cell, pairs positions with `for j in range(i + 1, len(list) - 1)`, which never reaches the last member, and for eq-diff2 and eq-diff3 skips zi = zj; the W3C rules pair every two positions i < j of the list, with every rdf:first of each cell, so `(a b c)` with `a owl:sameAs b` gives (a, a), (a, b), (b, a) and (b, b), and `(p q)` gives prp-adp)

```
! missing eq-diff2 <http://example.org/d> <http://example.org/b> <http://example.org/a>
! missing eq-diff2 <http://example.org/d> <http://example.org/a> <http://example.org/a>
! missing eq-diff2 <http://example.org/d> <http://example.org/b> <http://example.org/b>
```

### eq-diff2 (complete): OWL-RL: expectations (known: F6, owlrl 7.6.2 reads the list of eq-diff2, eq-diff3 (OWLRL.py:416-436) and prp-adp (OWLRL.py:517-531) through rdflib's `items`, one rdf:first per cell, pairs positions with `for j in range(i + 1, len(list) - 1)`, which never reaches the last member, and for eq-diff2 and eq-diff3 skips zi = zj; the W3C rules pair every two positions i < j of the list, with every rdf:first of each cell, so `(a b c)` with `a owl:sameAs b` gives (a, a), (a, b), (b, a) and (b, b), and `(p q)` gives prp-adp)

```
! missing eq-diff2 <http://example.org/d> <http://example.org/a> <http://example.org/a>
! missing eq-diff2 <http://example.org/d> <http://example.org/b> <http://example.org/b>
! missing eq-diff2 <http://example.org/d> <http://example.org/b> <http://example.org/a>
```

### eq-diff3 (default): OWL-RL: expectations (known: F6, owlrl 7.6.2 reads the list of eq-diff2, eq-diff3 (OWLRL.py:416-436) and prp-adp (OWLRL.py:517-531) through rdflib's `items`, one rdf:first per cell, pairs positions with `for j in range(i + 1, len(list) - 1)`, which never reaches the last member, and for eq-diff2 and eq-diff3 skips zi = zj; the W3C rules pair every two positions i < j of the list, with every rdf:first of each cell, so `(a b c)` with `a owl:sameAs b` gives (a, a), (a, b), (b, a) and (b, b), and `(p q)` gives prp-adp)

```
! missing eq-diff3 <http://example.org/d> <http://example.org/b> <http://example.org/b>
! missing eq-diff3 <http://example.org/d> <http://example.org/b> <http://example.org/a>
! missing eq-diff3 <http://example.org/d> <http://example.org/a> <http://example.org/a>
```

### eq-diff3 (complete): OWL-RL: expectations (known: F6, owlrl 7.6.2 reads the list of eq-diff2, eq-diff3 (OWLRL.py:416-436) and prp-adp (OWLRL.py:517-531) through rdflib's `items`, one rdf:first per cell, pairs positions with `for j in range(i + 1, len(list) - 1)`, which never reaches the last member, and for eq-diff2 and eq-diff3 skips zi = zj; the W3C rules pair every two positions i < j of the list, with every rdf:first of each cell, so `(a b c)` with `a owl:sameAs b` gives (a, a), (a, b), (b, a) and (b, b), and `(p q)` gives prp-adp)

```
! missing eq-diff3 <http://example.org/d> <http://example.org/a> <http://example.org/a>
! missing eq-diff3 <http://example.org/d> <http://example.org/b> <http://example.org/a>
! missing eq-diff3 <http://example.org/d> <http://example.org/b> <http://example.org/b>
```

### dt-not-type (default): OWL-RL: expectations (known: F3, owlrl 7.6.2 accepts " 42"^^xsd:int (Python's int() strips the spaces); RDF does not collapse whitespace, and the W3C test rdf-mt xmlsch-02-whitespace-facet-2 expects " 3 "^^xsd:int to be ill-typed)

```
! missing dt-not-type " 42"^^<http://www.w3.org/2001/XMLSchema#int> <http://www.w3.org/2001/XMLSchema#int>
```

### dt-not-type (complete): OWL-RL: expectations (known: F3, owlrl 7.6.2 accepts " 42"^^xsd:int (Python's int() strips the spaces); RDF does not collapse whitespace, and the W3C test rdf-mt xmlsch-02-whitespace-facet-2 expects " 3 "^^xsd:int to be ill-typed)

```
! missing dt-not-type " 42"^^<http://www.w3.org/2001/XMLSchema#int> <http://www.w3.org/2001/XMLSchema#int>
```

### dt-not-type-range (default): OWL-RL: expectations (known: F5, owlrl 7.6.2 runs dt-not-type only on literals against their own datatype (OWLRL.py:217-232), never on `lt rdf:type dt` triples, so a literal typed by prp-rng is not checked; the W3C dt-not-type matches any such triple, and the W3C test rdf-mt datatypes-range-clash expects "25"^^xsd:integer in an xsd:string range to be inconsistent)

```
! missing dt-not-type "x"@en <http://www.w3.org/2001/XMLSchema#integer>
! missing dt-not-type "abc" <http://www.w3.org/2001/XMLSchema#integer>
! missing dt-not-type "300"^^<http://www.w3.org/2001/XMLSchema#integer> <http://www.w3.org/2001/XMLSchema#byte>
```

### dt-not-type-range (complete): OWL-RL: expectations (known: F5, owlrl 7.6.2 runs dt-not-type only on literals against their own datatype (OWLRL.py:217-232), never on `lt rdf:type dt` triples, so a literal typed by prp-rng is not checked; the W3C dt-not-type matches any such triple, and the W3C test rdf-mt datatypes-range-clash expects "25"^^xsd:integer in an xsd:string range to be inconsistent)

```
! missing dt-not-type "x"@en <http://www.w3.org/2001/XMLSchema#integer>
! missing dt-not-type "abc" <http://www.w3.org/2001/XMLSchema#integer>
! missing dt-not-type "300"^^<http://www.w3.org/2001/XMLSchema#integer> <http://www.w3.org/2001/XMLSchema#byte>
```

### dt-not-type-owl-2-rl (default): OWL-RL: expectations (known: F5, owlrl 7.6.2 runs dt-not-type only on literals against their own datatype (OWLRL.py:217-232), never on `lt rdf:type dt` triples, so a literal typed by prp-rng is not checked; the W3C dt-not-type matches any such triple, and the W3C test rdf-mt datatypes-range-clash expects "25"^^xsd:integer in an xsd:string range to be inconsistent)

```
! missing dt-not-type "chat"@fr <http://www.w3.org/2001/XMLSchema#string>
```

### dt-not-type-owl-2-rl (complete): OWL-RL: expectations (known: F5, owlrl 7.6.2 runs dt-not-type only on literals against their own datatype (OWLRL.py:217-232), never on `lt rdf:type dt` triples, so a literal typed by prp-rng is not checked; the W3C dt-not-type matches any such triple, and the W3C test rdf-mt datatypes-range-clash expects "25"^^xsd:integer in an xsd:string range to be inconsistent)

```
! missing dt-not-type "chat"@fr <http://www.w3.org/2001/XMLSchema#string>
```

### eq-diff2-eq-ref (complete): OWL-RL: expectations (known: F6, owlrl 7.6.2 reads the list of eq-diff2, eq-diff3 (OWLRL.py:416-436) and prp-adp (OWLRL.py:517-531) through rdflib's `items`, one rdf:first per cell, pairs positions with `for j in range(i + 1, len(list) - 1)`, which never reaches the last member, and for eq-diff2 and eq-diff3 skips zi = zj; the W3C rules pair every two positions i < j of the list, with every rdf:first of each cell, so `(a b c)` with `a owl:sameAs b` gives (a, a), (a, b), (b, a) and (b, b), and `(p q)` gives prp-adp)

```
! missing eq-diff2 <http://example.org/d> <http://example.org/a> <http://example.org/a>
! missing eq-diff3 <http://example.org/e> <http://example.org/c> <http://example.org/c>
```

### dt-diff (complete): OWL-RL: expectations (known: F15, owlrl 7.6.2 skips dt-eq and dt-diff (OWLRL.py:234-237: "This rules are skipped on purpose at the moment"), and its dt-type2 types a literal of a datatype of its list with that datatype (OWLRL.py:215-220) and the supertypes its table gives (OWLRL.py:191-198, XsdDatatypes.py:85: xsd:int's are xsd:long, xsd:integer and xsd:decimal), a term made owl:sameAs the literal too (OWLRL.py:248-256). OWL 2 Profiles, Table 8: dt-eq makes every two literals with the same value owl:sameAs ("1"^^xsd:integer and "1.0"^^xsd:decimal), dt-diff makes those with different values owl:differentFrom (so two made owl:sameAs are inconsistent, eq-diff1), and dt-type2 types a literal with every datatype whose value space holds its value ("5"^^xsd:int is an xsd:byte and an xsd:unsignedByte))

```
! missing eq-diff1 "1"^^<http://www.w3.org/2001/XMLSchema#integer> "1"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger>
! missing eq-diff1 "1"^^<http://www.w3.org/2001/XMLSchema#integer> "2"^^<http://www.w3.org/2001/XMLSchema#integer>
! missing eq-diff1 "1"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger> "2"^^<http://www.w3.org/2001/XMLSchema#integer>
! missing eq-diff1 "1"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger> "1"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger>
! missing eq-diff1 "2"^^<http://www.w3.org/2001/XMLSchema#integer> "1"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger>
! missing eq-diff1 "2"^^<http://www.w3.org/2001/XMLSchema#integer> "1"^^<http://www.w3.org/2001/XMLSchema#integer>
! missing eq-diff1 "1"^^<http://www.w3.org/2001/XMLSchema#integer> "1"^^<http://www.w3.org/2001/XMLSchema#integer>
! missing eq-diff1 "2"^^<http://www.w3.org/2001/XMLSchema#integer> "2"^^<http://www.w3.org/2001/XMLSchema#integer>
! missing eq-diff1 "1"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger> "1"^^<http://www.w3.org/2001/XMLSchema#integer>
```

Not run:
- failed: OWL-RL: exit status: 1:     for cl in self._list(x):
              ~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
- failed: OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 344, in _property_chain
    chain = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
- failed: OWL-RL: exit status: 1:   File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 659, in _classes
    classes = self._list(x)
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/owlrl/OWLRL.py", line 125, in _list
    return [ch for ch in self.graph.items(l)]
                         ~~~~~~~~~~~~~~~~^^^
  File "/Users/abhishek/repos/turtle-reasoner/oracles/owlrl/.venv/lib/python3.14/site-packages/rdflib/graph.py", line 1222, in items
    raise ValueError("List contains a recursive rdf:rest reference")
ValueError: List contains a recursive rdf:rest reference
- HermiT entails Nemo's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (1 inferred subsumptions, such as <http://example.org/A> ⊑ <http://example.org/C>, name a term the input does not declare as owl:Class, owl:ObjectProperty or owl:DatatypeProperty)
- HermiT entails Nemo's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (2 inferred subsumptions, such as <http://example.org/A> ⊑ <http://example.org/A>, name a term the input does not declare as owl:Class, owl:ObjectProperty or owl:DatatypeProperty)
- HermiT entails Nemo's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (4 inferred subsumptions, such as <http://example.org/A> ⊑ <http://example.org/A>, name a term the input does not declare as owl:Class, owl:ObjectProperty or owl:DatatypeProperty)
- HermiT entails Nemo's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (<http://example.org/T> <http://www.w3.org/2002/07/owl#sameAs> <http://www.w3.org/2002/07/owl#Thing> makes a reserved IRI equal to another term)
- HermiT entails Nemo's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (<http://example.org/n> <http://www.w3.org/2002/07/owl#sameAs> <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> makes a reserved IRI equal to another term)
- HermiT entails Nemo's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (<http://example.org/same> <http://www.w3.org/2002/07/owl#sameAs> <http://www.w3.org/2002/07/owl#sameAs> makes a reserved IRI equal to another term)
- HermiT entails Nemo's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (<http://example.org/t> <http://www.w3.org/2002/07/owl#sameAs> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> makes a reserved IRI equal to another term)
- HermiT entails Nemo's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://www.w3.org/2002/07/owl#sameAs> <http://www.w3.org/2002/07/owl#equivalentClass> makes a reserved IRI equal to another term)
- HermiT entails ours's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (1 inferred subsumptions, such as <http://example.org/A> ⊑ <http://example.org/C>, name a term the input does not declare as owl:Class, owl:ObjectProperty or owl:DatatypeProperty)
- HermiT entails ours's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (2 inferred subsumptions, such as <http://example.org/A> ⊑ <http://example.org/A>, name a term the input does not declare as owl:Class, owl:ObjectProperty or owl:DatatypeProperty)
- HermiT entails ours's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (4 inferred subsumptions, such as <http://example.org/A> ⊑ <http://example.org/A>, name a term the input does not declare as owl:Class, owl:ObjectProperty or owl:DatatypeProperty)
- HermiT entails ours's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (<http://example.org/T> <http://www.w3.org/2002/07/owl#sameAs> <http://www.w3.org/2002/07/owl#Thing> makes a reserved IRI equal to another term)
- HermiT entails ours's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (<http://example.org/n> <http://www.w3.org/2002/07/owl#sameAs> <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> makes a reserved IRI equal to another term)
- HermiT entails ours's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (<http://example.org/same> <http://www.w3.org/2002/07/owl#sameAs> <http://www.w3.org/2002/07/owl#sameAs> makes a reserved IRI equal to another term)
- HermiT entails ours's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (<http://example.org/t> <http://www.w3.org/2002/07/owl#sameAs> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> makes a reserved IRI equal to another term)
- HermiT entails ours's subsumptions: skipped: HermiT: not OWL 2 DL, not judged (<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://www.w3.org/2002/07/owl#sameAs> <http://www.w3.org/2002/07/owl#equivalentClass> makes a reserved IRI equal to another term)
