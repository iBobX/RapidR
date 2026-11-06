' typed_arrays_many.bas: 130 typed arrays in the main program. Native builds
' keep them in thread-local statics, in blocks of at most 64: one block of
' 128 or more reached rustc's macro recursion limit and didn't build.
' Each array keeps its own values.
DIM T001(3) AS INTEGER
DIM T002(3) AS INTEGER
DIM T003(3) AS INTEGER
DIM T004(3) AS INTEGER
DIM T005(3) AS INTEGER
DIM T006(3) AS INTEGER
DIM T007(3) AS INTEGER
DIM T008(3) AS INTEGER
DIM T009(3) AS INTEGER
DIM T010(3) AS INTEGER
DIM T011(3) AS INTEGER
DIM T012(3) AS INTEGER
DIM T013(3) AS INTEGER
DIM T014(3) AS INTEGER
DIM T015(3) AS INTEGER
DIM T016(3) AS INTEGER
DIM T017(3) AS INTEGER
DIM T018(3) AS INTEGER
DIM T019(3) AS INTEGER
DIM T020(3) AS INTEGER
DIM T021(3) AS INTEGER
DIM T022(3) AS INTEGER
DIM T023(3) AS INTEGER
DIM T024(3) AS INTEGER
DIM T025(3) AS INTEGER
DIM T026(3) AS INTEGER
DIM T027(3) AS INTEGER
DIM T028(3) AS INTEGER
DIM T029(3) AS INTEGER
DIM T030(3) AS INTEGER
DIM T031(3) AS INTEGER
DIM T032(3) AS INTEGER
DIM T033(3) AS INTEGER
DIM T034(3) AS INTEGER
DIM T035(3) AS INTEGER
DIM T036(3) AS INTEGER
DIM T037(3) AS INTEGER
DIM T038(3) AS INTEGER
DIM T039(3) AS INTEGER
DIM T040(3) AS INTEGER
DIM T041(3) AS INTEGER
DIM T042(3) AS INTEGER
DIM T043(3) AS INTEGER
DIM T044(3) AS INTEGER
DIM T045(3) AS INTEGER
DIM T046(3) AS INTEGER
DIM T047(3) AS INTEGER
DIM T048(3) AS INTEGER
DIM T049(3) AS INTEGER
DIM T050(3) AS INTEGER
DIM T051(3) AS INTEGER
DIM T052(3) AS INTEGER
DIM T053(3) AS INTEGER
DIM T054(3) AS INTEGER
DIM T055(3) AS INTEGER
DIM T056(3) AS INTEGER
DIM T057(3) AS INTEGER
DIM T058(3) AS INTEGER
DIM T059(3) AS INTEGER
DIM T060(3) AS INTEGER
DIM T061(3) AS INTEGER
DIM T062(3) AS INTEGER
DIM T063(3) AS INTEGER
DIM T064(3) AS INTEGER
DIM T065(3) AS INTEGER
DIM T066(3) AS INTEGER
DIM T067(3) AS INTEGER
DIM T068(3) AS INTEGER
DIM T069(3) AS INTEGER
DIM T070(3) AS INTEGER
DIM T071(3) AS INTEGER
DIM T072(3) AS INTEGER
DIM T073(3) AS INTEGER
DIM T074(3) AS INTEGER
DIM T075(3) AS INTEGER
DIM T076(3) AS INTEGER
DIM T077(3) AS INTEGER
DIM T078(3) AS INTEGER
DIM T079(3) AS INTEGER
DIM T080(3) AS INTEGER
DIM T081(3) AS INTEGER
DIM T082(3) AS INTEGER
DIM T083(3) AS INTEGER
DIM T084(3) AS INTEGER
DIM T085(3) AS INTEGER
DIM T086(3) AS INTEGER
DIM T087(3) AS INTEGER
DIM T088(3) AS INTEGER
DIM T089(3) AS INTEGER
DIM T090(3) AS INTEGER
DIM T091(3) AS INTEGER
DIM T092(3) AS INTEGER
DIM T093(3) AS INTEGER
DIM T094(3) AS INTEGER
DIM T095(3) AS INTEGER
DIM T096(3) AS INTEGER
DIM T097(3) AS INTEGER
DIM T098(3) AS INTEGER
DIM T099(3) AS INTEGER
DIM T100(3) AS INTEGER
DIM T101(3) AS INTEGER
DIM T102(3) AS INTEGER
DIM T103(3) AS INTEGER
DIM T104(3) AS INTEGER
DIM T105(3) AS INTEGER
DIM T106(3) AS INTEGER
DIM T107(3) AS INTEGER
DIM T108(3) AS INTEGER
DIM T109(3) AS INTEGER
DIM T110(3) AS INTEGER
DIM T111(3) AS INTEGER
DIM T112(3) AS INTEGER
DIM T113(3) AS INTEGER
DIM T114(3) AS INTEGER
DIM T115(3) AS INTEGER
DIM T116(3) AS INTEGER
DIM T117(3) AS INTEGER
DIM T118(3) AS INTEGER
DIM T119(3) AS INTEGER
DIM T120(3) AS INTEGER
DIM T121(3) AS INTEGER
DIM T122(3) AS INTEGER
DIM T123(3) AS INTEGER
DIM T124(3) AS INTEGER
DIM T125(3) AS INTEGER
DIM T126(3) AS INTEGER
DIM T127(3) AS INTEGER
DIM T128(3) AS INTEGER
DIM T129(3) AS INTEGER
DIM T130(3) AS INTEGER
DIM total AS LONG
T001(2) = 1
T002(2) = 2
T003(2) = 3
T004(2) = 4
T005(2) = 5
T006(2) = 6
T007(2) = 7
T008(2) = 8
T009(2) = 9
T010(2) = 10
T011(2) = 11
T012(2) = 12
T013(2) = 13
T014(2) = 14
T015(2) = 15
T016(2) = 16
T017(2) = 17
T018(2) = 18
T019(2) = 19
T020(2) = 20
T021(2) = 21
T022(2) = 22
T023(2) = 23
T024(2) = 24
T025(2) = 25
T026(2) = 26
T027(2) = 27
T028(2) = 28
T029(2) = 29
T030(2) = 30
T031(2) = 31
T032(2) = 32
T033(2) = 33
T034(2) = 34
T035(2) = 35
T036(2) = 36
T037(2) = 37
T038(2) = 38
T039(2) = 39
T040(2) = 40
T041(2) = 41
T042(2) = 42
T043(2) = 43
T044(2) = 44
T045(2) = 45
T046(2) = 46
T047(2) = 47
T048(2) = 48
T049(2) = 49
T050(2) = 50
T051(2) = 51
T052(2) = 52
T053(2) = 53
T054(2) = 54
T055(2) = 55
T056(2) = 56
T057(2) = 57
T058(2) = 58
T059(2) = 59
T060(2) = 60
T061(2) = 61
T062(2) = 62
T063(2) = 63
T064(2) = 64
T065(2) = 65
T066(2) = 66
T067(2) = 67
T068(2) = 68
T069(2) = 69
T070(2) = 70
T071(2) = 71
T072(2) = 72
T073(2) = 73
T074(2) = 74
T075(2) = 75
T076(2) = 76
T077(2) = 77
T078(2) = 78
T079(2) = 79
T080(2) = 80
T081(2) = 81
T082(2) = 82
T083(2) = 83
T084(2) = 84
T085(2) = 85
T086(2) = 86
T087(2) = 87
T088(2) = 88
T089(2) = 89
T090(2) = 90
T091(2) = 91
T092(2) = 92
T093(2) = 93
T094(2) = 94
T095(2) = 95
T096(2) = 96
T097(2) = 97
T098(2) = 98
T099(2) = 99
T100(2) = 100
T101(2) = 101
T102(2) = 102
T103(2) = 103
T104(2) = 104
T105(2) = 105
T106(2) = 106
T107(2) = 107
T108(2) = 108
T109(2) = 109
T110(2) = 110
T111(2) = 111
T112(2) = 112
T113(2) = 113
T114(2) = 114
T115(2) = 115
T116(2) = 116
T117(2) = 117
T118(2) = 118
T119(2) = 119
T120(2) = 120
T121(2) = 121
T122(2) = 122
T123(2) = 123
T124(2) = 124
T125(2) = 125
T126(2) = 126
T127(2) = 127
T128(2) = 128
T129(2) = 129
T130(2) = 130
total = total + T001(2)
total = total + T002(2)
total = total + T003(2)
total = total + T004(2)
total = total + T005(2)
total = total + T006(2)
total = total + T007(2)
total = total + T008(2)
total = total + T009(2)
total = total + T010(2)
total = total + T011(2)
total = total + T012(2)
total = total + T013(2)
total = total + T014(2)
total = total + T015(2)
total = total + T016(2)
total = total + T017(2)
total = total + T018(2)
total = total + T019(2)
total = total + T020(2)
total = total + T021(2)
total = total + T022(2)
total = total + T023(2)
total = total + T024(2)
total = total + T025(2)
total = total + T026(2)
total = total + T027(2)
total = total + T028(2)
total = total + T029(2)
total = total + T030(2)
total = total + T031(2)
total = total + T032(2)
total = total + T033(2)
total = total + T034(2)
total = total + T035(2)
total = total + T036(2)
total = total + T037(2)
total = total + T038(2)
total = total + T039(2)
total = total + T040(2)
total = total + T041(2)
total = total + T042(2)
total = total + T043(2)
total = total + T044(2)
total = total + T045(2)
total = total + T046(2)
total = total + T047(2)
total = total + T048(2)
total = total + T049(2)
total = total + T050(2)
total = total + T051(2)
total = total + T052(2)
total = total + T053(2)
total = total + T054(2)
total = total + T055(2)
total = total + T056(2)
total = total + T057(2)
total = total + T058(2)
total = total + T059(2)
total = total + T060(2)
total = total + T061(2)
total = total + T062(2)
total = total + T063(2)
total = total + T064(2)
total = total + T065(2)
total = total + T066(2)
total = total + T067(2)
total = total + T068(2)
total = total + T069(2)
total = total + T070(2)
total = total + T071(2)
total = total + T072(2)
total = total + T073(2)
total = total + T074(2)
total = total + T075(2)
total = total + T076(2)
total = total + T077(2)
total = total + T078(2)
total = total + T079(2)
total = total + T080(2)
total = total + T081(2)
total = total + T082(2)
total = total + T083(2)
total = total + T084(2)
total = total + T085(2)
total = total + T086(2)
total = total + T087(2)
total = total + T088(2)
total = total + T089(2)
total = total + T090(2)
total = total + T091(2)
total = total + T092(2)
total = total + T093(2)
total = total + T094(2)
total = total + T095(2)
total = total + T096(2)
total = total + T097(2)
total = total + T098(2)
total = total + T099(2)
total = total + T100(2)
total = total + T101(2)
total = total + T102(2)
total = total + T103(2)
total = total + T104(2)
total = total + T105(2)
total = total + T106(2)
total = total + T107(2)
total = total + T108(2)
total = total + T109(2)
total = total + T110(2)
total = total + T111(2)
total = total + T112(2)
total = total + T113(2)
total = total + T114(2)
total = total + T115(2)
total = total + T116(2)
total = total + T117(2)
total = total + T118(2)
total = total + T119(2)
total = total + T120(2)
total = total + T121(2)
total = total + T122(2)
total = total + T123(2)
total = total + T124(2)
total = total + T125(2)
total = total + T126(2)
total = total + T127(2)
total = total + T128(2)
total = total + T129(2)
total = total + T130(2)
PRINT "arrays: 130, sum:"; total
PRINT "first:"; T001(2); ", last:"; T130(2)
