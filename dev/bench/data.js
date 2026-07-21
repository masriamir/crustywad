window.BENCHMARK_DATA = {
  "lastUpdate": 1784594033973,
  "repoUrl": "https://github.com/masriamir/crustywad",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "59276103dfd76939fdd1a41d6b6f0d9bb53d4f01",
          "message": "fix(ci): exclude lib from bench mode and update crossbeam-epoch (#167)",
          "timestamp": "2026-07-06T19:11:01-04:00",
          "tree_id": "1564f87bee6bf2f4202e25e3538bbf42671706c0",
          "url": "https://github.com/masriamir/crustywad/commit/59276103dfd76939fdd1a41d6b6f0d9bb53d4f01"
        },
        "date": 1783380042075,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 361,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3779,
            "range": "± 930",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 36966,
            "range": "± 1074",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 372,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3789,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38307,
            "range": "± 1822",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28880,
            "range": "± 187",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28924,
            "range": "± 372",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22588,
            "range": "± 122",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22544,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15812,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1074,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11384,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18901,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 24597,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5331,
            "range": "± 89",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11805,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5326,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22087,
            "range": "± 125",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23048,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 322,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 13982,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 737174,
            "range": "± 31402",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 325,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 13514,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 749955,
            "range": "± 16542",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 896,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 30409,
            "range": "± 160",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1353,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 47812,
            "range": "± 932",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2599049,
            "range": "± 42674",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "35fb00cd570abfe6fff40406538a5807ec3b8e39",
          "message": "docs(adr): fix status drift for ADR-0007, 0008, 0009, 0010, 0011 (#169)",
          "timestamp": "2026-07-06T20:08:58-04:00",
          "tree_id": "fd452e427941f43793782b437518a94c06c0af2f",
          "url": "https://github.com/masriamir/crustywad/commit/35fb00cd570abfe6fff40406538a5807ec3b8e39"
        },
        "date": 1783383440656,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 368,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4493,
            "range": "± 76",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 36534,
            "range": "± 2746",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 388,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4606,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37059,
            "range": "± 902",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 30056,
            "range": "± 280",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 30238,
            "range": "± 162",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20327,
            "range": "± 149",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20359,
            "range": "± 119",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18093,
            "range": "± 528",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1070,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10580,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19413,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 22388,
            "range": "± 149",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4718,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10999,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4717,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20933,
            "range": "± 76",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23338,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 318,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16928,
            "range": "± 148",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 729505,
            "range": "± 42144",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 313,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14438,
            "range": "± 54",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 726792,
            "range": "± 12818",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 897,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 35189,
            "range": "± 368",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1380,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 47080,
            "range": "± 1700",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2109729,
            "range": "± 45535",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "807293f54ecb9aacdbd0618f13f311ecb0917bcb",
          "message": "chore(release): add crates.io metadata and verify publish dry-run (#173)",
          "timestamp": "2026-07-06T21:19:42-04:00",
          "tree_id": "0e177b512de32f68c5856d2f5721e733fe2f343e",
          "url": "https://github.com/masriamir/crustywad/commit/807293f54ecb9aacdbd0618f13f311ecb0917bcb"
        },
        "date": 1783387666260,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 276,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 2849,
            "range": "± 311",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 28513,
            "range": "± 1996",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 288,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 2942,
            "range": "± 297",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 30099,
            "range": "± 1325",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 22707,
            "range": "± 356",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 22477,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 17731,
            "range": "± 330",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 17587,
            "range": "± 141",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 31,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 11634,
            "range": "± 308",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 826,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 8830,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 14641,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 19391,
            "range": "± 1002",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4132,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 9160,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4135,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 17134,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 17926,
            "range": "± 332",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 270,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 10950,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 593888,
            "range": "± 6688",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 264,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 11056,
            "range": "± 195",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 590059,
            "range": "± 26082",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 678,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 25086,
            "range": "± 94",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1037,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 36014,
            "range": "± 533",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2212006,
            "range": "± 32093",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e0bbd13304deb9748c3526d4429676670e593061",
          "message": "docs: fix milestone-1-only staleness across guide, CLAUDE.md, copilot-instructions.md, design.md (#175)",
          "timestamp": "2026-07-06T22:43:30-04:00",
          "tree_id": "36eddfe6ca8c11ffd0fcf40a0335e673df865e40",
          "url": "https://github.com/masriamir/crustywad/commit/e0bbd13304deb9748c3526d4429676670e593061"
        },
        "date": 1783392710934,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 276,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3053,
            "range": "± 92",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 28922,
            "range": "± 1127",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 289,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 2948,
            "range": "± 77",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 30008,
            "range": "± 1582",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 29596,
            "range": "± 284",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 29568,
            "range": "± 120",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 16713,
            "range": "± 91",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 16767,
            "range": "± 77",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 31,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 12058,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 918,
            "range": "± 367",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 8812,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 14626,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 19094,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4133,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 9141,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4134,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 17129,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 17859,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 254,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 10723,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 608365,
            "range": "± 5881",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 258,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 10456,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 608501,
            "range": "± 11681",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 671,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 23737,
            "range": "± 118",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1072,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 38198,
            "range": "± 1176",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2209807,
            "range": "± 21506",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7949d83d201effeb7ab7d5e4f9197e5a52b45ad6",
          "message": "docs: add write-path guide page, runnable examples/, and expand write-path doctests (#177)",
          "timestamp": "2026-07-06T23:10:24-04:00",
          "tree_id": "726259c6b757512e4d3c0e209526496d0d040803",
          "url": "https://github.com/masriamir/crustywad/commit/7949d83d201effeb7ab7d5e4f9197e5a52b45ad6"
        },
        "date": 1783394335598,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 368,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4308,
            "range": "± 171",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 36840,
            "range": "± 978",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 384,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4295,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38310,
            "range": "± 1465",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 29112,
            "range": "± 648",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 29104,
            "range": "± 131",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20410,
            "range": "± 160",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20267,
            "range": "± 130",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18112,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1079,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10577,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19358,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 22356,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4712,
            "range": "± 115",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10966,
            "range": "± 132",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4716,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20866,
            "range": "± 250",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23221,
            "range": "± 182",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 314,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 15460,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 737129,
            "range": "± 23849",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 321,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 13722,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 739449,
            "range": "± 8383",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 886,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 34027,
            "range": "± 135",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1324,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 48636,
            "range": "± 574",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 1933586,
            "range": "± 38528",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7cbc589efc9dca94bd9220c3578424794bd1aa28",
          "message": "chore(release): implement independent versioning migration (ADR-0011 §3) (#171)",
          "timestamp": "2026-07-06T23:48:22-04:00",
          "tree_id": "016a7e671aba78b97cd45282f65e00d87ef56068",
          "url": "https://github.com/masriamir/crustywad/commit/7cbc589efc9dca94bd9220c3578424794bd1aa28"
        },
        "date": 1783396591495,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 338,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3720,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 40543,
            "range": "± 967",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 389,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3739,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 39273,
            "range": "± 1388",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 19317,
            "range": "± 87",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 16882,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 14691,
            "range": "± 98",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 14770,
            "range": "± 130",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 3,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 14038,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1281,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10469,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 15761,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 21281,
            "range": "± 94",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 3434,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10839,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 3446,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 18652,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 18957,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 275,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 11974,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 1563746,
            "range": "± 135900",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 275,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 11238,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 1518020,
            "range": "± 110949",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 938,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 32863,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1508,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 55357,
            "range": "± 3884",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 4689768,
            "range": "± 194714",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e2a4e17391e21d9b30747f7e6f2aee9957d7efa9",
          "message": "docs: overhaul README.md as a concise crates.io landing page (#179)",
          "timestamp": "2026-07-07T00:18:08-04:00",
          "tree_id": "f05c7a2aae4263111280f099c816ca59cb701697",
          "url": "https://github.com/masriamir/crustywad/commit/e2a4e17391e21d9b30747f7e6f2aee9957d7efa9"
        },
        "date": 1783398403335,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 368,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4150,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37570,
            "range": "± 814",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 384,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4334,
            "range": "± 138",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38436,
            "range": "± 921",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 26959,
            "range": "± 965",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28465,
            "range": "± 1217",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21545,
            "range": "± 358",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21621,
            "range": "± 196",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 14415,
            "range": "± 86",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1078,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10553,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17510,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 24260,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4721,
            "range": "± 367",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10970,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4722,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20584,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 25405,
            "range": "± 61",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 314,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 17994,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 904234,
            "range": "± 70057",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 318,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14183,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 888923,
            "range": "± 33609",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 920,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 39477,
            "range": "± 379",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1377,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 47581,
            "range": "± 3202",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2407310,
            "range": "± 65646",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9bca2440976f1f6539f98159f88169249937fa15",
          "message": "chore: release v0.1.0 (#7)",
          "timestamp": "2026-07-07T11:03:16-04:00",
          "tree_id": "1f418e3e61b3cfb48c2100ff46678f8ae27bec00",
          "url": "https://github.com/masriamir/crustywad/commit/9bca2440976f1f6539f98159f88169249937fa15"
        },
        "date": 1783437109315,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 368,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4353,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37486,
            "range": "± 1264",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 384,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4346,
            "range": "± 77",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37938,
            "range": "± 1160",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28622,
            "range": "± 277",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28722,
            "range": "± 989",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22071,
            "range": "± 2502",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22110,
            "range": "± 276",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18988,
            "range": "± 145",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1061,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10534,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17425,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 24822,
            "range": "± 1115",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4729,
            "range": "± 70",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10689,
            "range": "± 226",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4717,
            "range": "± 784",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20200,
            "range": "± 97",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 22650,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 315,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16565,
            "range": "± 94",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 774045,
            "range": "± 13179",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 313,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 15744,
            "range": "± 449",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 713820,
            "range": "± 16153",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 932,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 35813,
            "range": "± 541",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1358,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 47990,
            "range": "± 1246",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2278062,
            "range": "± 166454",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8b80058a310ae216fe04e77189f488d26c094b44",
          "message": "ci(release): enable crates.io publishing via Trusted Publishing (OIDC) (#182)",
          "timestamp": "2026-07-07T11:40:29-04:00",
          "tree_id": "6c9c58e3c7bf13b8c728189ad136e233cc2e1ae3",
          "url": "https://github.com/masriamir/crustywad/commit/8b80058a310ae216fe04e77189f488d26c094b44"
        },
        "date": 1783439341644,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 357,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3774,
            "range": "± 1308",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38160,
            "range": "± 7271",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 372,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3863,
            "range": "± 81",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 39568,
            "range": "± 1441",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28576,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28593,
            "range": "± 612",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21445,
            "range": "± 202",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21398,
            "range": "± 293",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15539,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1086,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11374,
            "range": "± 86",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19215,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 24799,
            "range": "± 127",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5326,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11785,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5326,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 23793,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 22897,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 322,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 14243,
            "range": "± 249",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 769520,
            "range": "± 63622",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 347,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 15079,
            "range": "± 262",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 748826,
            "range": "± 22620",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 904,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 33274,
            "range": "± 196",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1355,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 48868,
            "range": "± 1527",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2378638,
            "range": "± 24378",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "31404778e52b8771322a0ecc2214c86998a1716d",
          "message": "ci(release): ship cwad binaries via dist + GitHub App (#185)",
          "timestamp": "2026-07-07T23:27:03-04:00",
          "tree_id": "15c638f518afaeb1c75f1559e8c4e4d22bcd089c",
          "url": "https://github.com/masriamir/crustywad/commit/31404778e52b8771322a0ecc2214c86998a1716d"
        },
        "date": 1783481736914,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 367,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4276,
            "range": "± 261",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37344,
            "range": "± 791",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 380,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4431,
            "range": "± 101",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38056,
            "range": "± 1189",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 30492,
            "range": "± 295",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 30258,
            "range": "± 149",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21975,
            "range": "± 189",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22155,
            "range": "± 1151",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 19476,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1078,
            "range": "± 54",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10576,
            "range": "± 485",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19394,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 22465,
            "range": "± 131",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4722,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11016,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4718,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20915,
            "range": "± 662",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23355,
            "range": "± 86",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 315,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 18144,
            "range": "± 348",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 805059,
            "range": "± 23997",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 318,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 18186,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 781320,
            "range": "± 44785",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 932,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 35140,
            "range": "± 552",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1328,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 52533,
            "range": "± 1549",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2693590,
            "range": "± 97407",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2610c3300da19f1474593e8faa1e4541eb23e16f",
          "message": "chore: release v0.1.1 (#183)",
          "timestamp": "2026-07-07T23:41:05-04:00",
          "tree_id": "b1acab420cb7095482d9a1620e9e9fe4f7483abd",
          "url": "https://github.com/masriamir/crustywad/commit/2610c3300da19f1474593e8faa1e4541eb23e16f"
        },
        "date": 1783482581223,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 368,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3856,
            "range": "± 177",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38677,
            "range": "± 2853",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 384,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4297,
            "range": "± 401",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38532,
            "range": "± 971",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 26570,
            "range": "± 316",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 26624,
            "range": "± 256",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20813,
            "range": "± 469",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21404,
            "range": "± 235",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 17898,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1061,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10562,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17545,
            "range": "± 113",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 24276,
            "range": "± 511",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4720,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10976,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4719,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20560,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 25273,
            "range": "± 150",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 321,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16594,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 855457,
            "range": "± 13670",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 319,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 16129,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 836309,
            "range": "± 18305",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 905,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 34604,
            "range": "± 199",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1353,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 49905,
            "range": "± 1340",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2459594,
            "range": "± 49238",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "06158f168c39cfd6ca83e464206a1bd6d451d324",
          "message": "ci(release): title cwad releases as \"crustywad-cli X.Y.Z\" from tag (#187)",
          "timestamp": "2026-07-08T00:42:30-04:00",
          "tree_id": "084699455c71d7a1e1cfd152bf5b7b2154cac59c",
          "url": "https://github.com/masriamir/crustywad/commit/06158f168c39cfd6ca83e464206a1bd6d451d324"
        },
        "date": 1783486285244,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 366,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4181,
            "range": "± 61",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37097,
            "range": "± 705",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 382,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4308,
            "range": "± 199",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37993,
            "range": "± 940",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28045,
            "range": "± 119",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 29151,
            "range": "± 699",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20279,
            "range": "± 127",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20359,
            "range": "± 115",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 16300,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1049,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10555,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17538,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 24220,
            "range": "± 304",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4717,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11008,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4697,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20551,
            "range": "± 157",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 25274,
            "range": "± 91",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 323,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 15759,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 673200,
            "range": "± 20205",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 316,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 13653,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 664315,
            "range": "± 11490",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 953,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 35345,
            "range": "± 176",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1354,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 46363,
            "range": "± 688",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 1949492,
            "range": "± 39536",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "bbde05d52d5bfb9e596673a75195a0e04d2a86f6",
          "message": "ci: pin all GitHub Actions to commit SHAs (#189)",
          "timestamp": "2026-07-08T08:39:03-04:00",
          "tree_id": "f5bff71f33dfc40746b9a42a03ef584440fe6b5b",
          "url": "https://github.com/masriamir/crustywad/commit/bbde05d52d5bfb9e596673a75195a0e04d2a86f6"
        },
        "date": 1783514836789,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 275,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 2857,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 29775,
            "range": "± 2029",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 288,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 2924,
            "range": "± 102",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 30314,
            "range": "± 2712",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 21652,
            "range": "± 314",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 21759,
            "range": "± 224",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 17639,
            "range": "± 643",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 17595,
            "range": "± 135",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 31,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 12030,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 843,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 8545,
            "range": "± 111",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 14731,
            "range": "± 119",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 19204,
            "range": "± 276",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4135,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 9142,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4110,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 16833,
            "range": "± 214",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 17732,
            "range": "± 252",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 275,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 10166,
            "range": "± 218",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 688832,
            "range": "± 40514",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 258,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 11915,
            "range": "± 114",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 708145,
            "range": "± 52000",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 678,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 25833,
            "range": "± 847",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1051,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 37781,
            "range": "± 509",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2288778,
            "range": "± 209672",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@gmail.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "masriamir@gmail.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "distinct": true,
          "id": "2bc008503db182308f239491e6516666d60aa0da",
          "message": "chore: trigger CI to verify SHA-pinning enforcement",
          "timestamp": "2026-07-08T08:42:29-04:00",
          "tree_id": "f5bff71f33dfc40746b9a42a03ef584440fe6b5b",
          "url": "https://github.com/masriamir/crustywad/commit/2bc008503db182308f239491e6516666d60aa0da"
        },
        "date": 1783515352451,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 365,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4262,
            "range": "± 141",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37184,
            "range": "± 1687",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 383,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4314,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37982,
            "range": "± 889",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 30038,
            "range": "± 138",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 29735,
            "range": "± 213",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21504,
            "range": "± 549",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21777,
            "range": "± 144",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 14571,
            "range": "± 104",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1077,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10566,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17520,
            "range": "± 725",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 24267,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4719,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10977,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4718,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20636,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 25374,
            "range": "± 611",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 323,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 17185,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 763320,
            "range": "± 18104",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 317,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 13567,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 759313,
            "range": "± 13370",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 902,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 36659,
            "range": "± 372",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1365,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 50278,
            "range": "± 943",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2023665,
            "range": "± 28965",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@gmail.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "masriamir@gmail.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "distinct": true,
          "id": "5b0c336b734e560b850151e1a2deb37462113f8e",
          "message": "Revert \"chore: trigger CI to verify SHA-pinning enforcement\"\n\nThis reverts commit 2bc008503db182308f239491e6516666d60aa0da.",
          "timestamp": "2026-07-08T09:39:41-04:00",
          "tree_id": "f5bff71f33dfc40746b9a42a03ef584440fe6b5b",
          "url": "https://github.com/masriamir/crustywad/commit/5b0c336b734e560b850151e1a2deb37462113f8e"
        },
        "date": 1783518492599,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 369,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3834,
            "range": "± 255",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38173,
            "range": "± 1029",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 383,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4354,
            "range": "± 71",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38264,
            "range": "± 4475",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 29047,
            "range": "± 568",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28573,
            "range": "± 734",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20511,
            "range": "± 372",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20490,
            "range": "± 142",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15450,
            "range": "± 54",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1076,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10582,
            "range": "± 96",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19389,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 22435,
            "range": "± 247",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4736,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10999,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4716,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20913,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23370,
            "range": "± 183",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 318,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 17208,
            "range": "± 190",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 758422,
            "range": "± 29558",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 319,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14153,
            "range": "± 185",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 776285,
            "range": "± 49829",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 928,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 34934,
            "range": "± 497",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1400,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 50600,
            "range": "± 929",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2156210,
            "range": "± 48506",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "46595289b9c40b3534c10addc58e8c1f6c7a70a0",
          "message": "fix(bench): give large-input parse benchmarks more measurement time (#191)",
          "timestamp": "2026-07-08T14:57:15-04:00",
          "tree_id": "41611e57393617b3a2460228e4927f39608c0c0c",
          "url": "https://github.com/masriamir/crustywad/commit/46595289b9c40b3534c10addc58e8c1f6c7a70a0"
        },
        "date": 1783537514569,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 192,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 2508,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 26562,
            "range": "± 858",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 255,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 2481,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 26142,
            "range": "± 1161",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 13066,
            "range": "± 100",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 12901,
            "range": "± 134",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 15917,
            "range": "± 1340",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 15945,
            "range": "± 373",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 6768,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 817,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 6054,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 9885,
            "range": "± 218",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 15389,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 2201,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 6547,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 2195,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 12086,
            "range": "± 333",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 13150,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 165,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 5452,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 400918,
            "range": "± 24311",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 175,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 5246,
            "range": "± 329",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 407158,
            "range": "± 17530",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 592,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 17608,
            "range": "± 1574",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 848,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 25477,
            "range": "± 764",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 1183333,
            "range": "± 52511",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6b35c21421579914feb9129a5040c0698ca9f29b",
          "message": "docs(project): document tracking workflow and align branch-name hook (#197) (#198)",
          "timestamp": "2026-07-08T18:46:14-04:00",
          "tree_id": "0e9f50d8c19b536509a6657bc334867d0bd38bff",
          "url": "https://github.com/masriamir/crustywad/commit/6b35c21421579914feb9129a5040c0698ca9f29b"
        },
        "date": 1783551303134,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 366,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4383,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 36639,
            "range": "± 974",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 383,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4319,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37280,
            "range": "± 916",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27494,
            "range": "± 176",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27549,
            "range": "± 128",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21865,
            "range": "± 125",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21655,
            "range": "± 131",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18713,
            "range": "± 177",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1098,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10556,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19380,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 22338,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5841,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11404,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4717,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20908,
            "range": "± 91",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23239,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 318,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 17305,
            "range": "± 189",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 740828,
            "range": "± 24693",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 313,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14248,
            "range": "± 54",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 717955,
            "range": "± 53963",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 906,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 32629,
            "range": "± 190",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1375,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 51021,
            "range": "± 1295",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2078141,
            "range": "± 55445",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b9389d72fe0dbd31f42b916859fc23bbe4532785",
          "message": "docs(adr): add ADR-0014 multi-format map support strategy (#53) (#193)",
          "timestamp": "2026-07-08T20:33:37-04:00",
          "tree_id": "2b73f83fbce7479ac1fd6c78b4a5bc3764c4758e",
          "url": "https://github.com/masriamir/crustywad/commit/b9389d72fe0dbd31f42b916859fc23bbe4532785"
        },
        "date": 1783557731325,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 368,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4230,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37348,
            "range": "± 595",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 384,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4286,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37158,
            "range": "± 1554",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27631,
            "range": "± 161",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27244,
            "range": "± 457",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21583,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21503,
            "range": "± 477",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18104,
            "range": "± 770",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1080,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10524,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17615,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 24828,
            "range": "± 212",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4716,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10679,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4695,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20129,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 22558,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 317,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 17922,
            "range": "± 128",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 730687,
            "range": "± 18310",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 308,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 16362,
            "range": "± 204",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 737561,
            "range": "± 13400",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 902,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 39217,
            "range": "± 190",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1387,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 49047,
            "range": "± 312",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2381743,
            "range": "± 21850",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "17a0ff19a5d1b32e481409ab99b89aa75180e32e",
          "message": "docs(adr): add ADR-0015 assembled map graph model (#155) (#194)",
          "timestamp": "2026-07-08T22:48:40-04:00",
          "tree_id": "a5ffc8cfca49380cde43b5435e92fdf89b48b9af",
          "url": "https://github.com/masriamir/crustywad/commit/17a0ff19a5d1b32e481409ab99b89aa75180e32e"
        },
        "date": 1783565854211,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 355,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3835,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 36865,
            "range": "± 824",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 372,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3902,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 36239,
            "range": "± 1127",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28935,
            "range": "± 427",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28776,
            "range": "± 130",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21837,
            "range": "± 119",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21477,
            "range": "± 124",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15563,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1105,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11386,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18852,
            "range": "± 620",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 24658,
            "range": "± 318",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5323,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11809,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5326,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22082,
            "range": "± 327",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23091,
            "range": "± 74",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 328,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 13580,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 810679,
            "range": "± 34978",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 315,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 15210,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 809549,
            "range": "± 38718",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 899,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 31013,
            "range": "± 918",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1322,
            "range": "± 219",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 45414,
            "range": "± 799",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2585982,
            "range": "± 33027",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8b786f44548460ed7561aa4deaa7b8c003245bd3",
          "message": "docs(adr): add ADR-0016 parser and assembly hardening policy (#195) (#196)",
          "timestamp": "2026-07-08T23:16:09-04:00",
          "tree_id": "73cdafcd61c7655169034b47fa0e0723e1bd6341",
          "url": "https://github.com/masriamir/crustywad/commit/8b786f44548460ed7561aa4deaa7b8c003245bd3"
        },
        "date": 1783567482362,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 365,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4189,
            "range": "± 120",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37639,
            "range": "± 1541",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 383,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4293,
            "range": "± 76",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37381,
            "range": "± 1115",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27403,
            "range": "± 328",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27872,
            "range": "± 191",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20676,
            "range": "± 427",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20514,
            "range": "± 177",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 40,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 16340,
            "range": "± 530",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1053,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10607,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19404,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 22396,
            "range": "± 87",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4712,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11029,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4698,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20936,
            "range": "± 67",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23284,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 322,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 15627,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 793889,
            "range": "± 48905",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 316,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 13769,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 727287,
            "range": "± 48294",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 916,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 35380,
            "range": "± 130",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 2032,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 50090,
            "range": "± 1245",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2240518,
            "range": "± 56345",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "779e53897fe761cfe68fe03f8ada10f689d8aa7c",
          "message": "docs(adr): mark ADR-0015 & ADR-0016 Accepted; cite #199 in 0015 revisit condition (#200)",
          "timestamp": "2026-07-08T23:52:29-04:00",
          "tree_id": "90d3d6bdfa955184dbc310ef42cf5a309c80e684",
          "url": "https://github.com/masriamir/crustywad/commit/779e53897fe761cfe68fe03f8ada10f689d8aa7c"
        },
        "date": 1783569665660,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 365,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4311,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 36568,
            "range": "± 723",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 383,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4337,
            "range": "± 292",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38187,
            "range": "± 978",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27366,
            "range": "± 687",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27496,
            "range": "± 266",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21985,
            "range": "± 176",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21926,
            "range": "± 243",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 46,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 16098,
            "range": "± 85",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1069,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10593,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19427,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 22418,
            "range": "± 544",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4689,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11027,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4698,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20925,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23269,
            "range": "± 98",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 322,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16022,
            "range": "± 70",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 702263,
            "range": "± 10144",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 315,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14835,
            "range": "± 128",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 672977,
            "range": "± 25609",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 911,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 34781,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1387,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 45909,
            "range": "± 1281",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2079257,
            "range": "± 34939",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "72a9706cbca712cd5e810bf0e451a925aaf73bfe",
          "message": "docs: document agent-driven project-board status transitions (#203)",
          "timestamp": "2026-07-09T11:58:01-04:00",
          "tree_id": "3fb8a19720366604aa540c23c7f08f36d2837c54",
          "url": "https://github.com/masriamir/crustywad/commit/72a9706cbca712cd5e810bf0e451a925aaf73bfe"
        },
        "date": 1783613244210,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 389,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4235,
            "range": "± 154",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37694,
            "range": "± 821",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 400,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4384,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37531,
            "range": "± 720",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28306,
            "range": "± 676",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28287,
            "range": "± 248",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21562,
            "range": "± 132",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21623,
            "range": "± 243",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 16280,
            "range": "± 91",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1128,
            "range": "± 416",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10311,
            "range": "± 87",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17198,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 26066,
            "range": "± 110",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4726,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10661,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4723,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22891,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 22292,
            "range": "± 144",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 321,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 17594,
            "range": "± 137",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 789479,
            "range": "± 20553",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 319,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 17704,
            "range": "± 317",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 763907,
            "range": "± 19603",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 869,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 35653,
            "range": "± 221",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1380,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 51066,
            "range": "± 661",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2082097,
            "range": "± 42905",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "adf41236c72b64081ddeb64b13ecec6b40fb5159",
          "message": "refactor(map)!: split records into map::doom / map::common; consolidate name decode (#201) (#202)",
          "timestamp": "2026-07-09T14:42:03-04:00",
          "tree_id": "cb41c0a4302b9cc52c7cb0a88fe3d583b78c5f8d",
          "url": "https://github.com/masriamir/crustywad/commit/adf41236c72b64081ddeb64b13ecec6b40fb5159"
        },
        "date": 1783623065736,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 375,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3669,
            "range": "± 77",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 36805,
            "range": "± 993",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 392,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3773,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37825,
            "range": "± 1481",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28344,
            "range": "± 71",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28381,
            "range": "± 447",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22645,
            "range": "± 283",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22683,
            "range": "± 171",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 40,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15913,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1079,
            "range": "± 86",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11367,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18919,
            "range": "± 117",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 32668,
            "range": "± 98",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5340,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11790,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5328,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22978,
            "range": "± 133",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23490,
            "range": "± 438",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 339,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 13764,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 700058,
            "range": "± 24373",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 305,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 13870,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 701697,
            "range": "± 59349",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 857,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 36186,
            "range": "± 116",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1349,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 44481,
            "range": "± 1248",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2341343,
            "range": "± 22460",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2b634eea78652706ead1e7ff34a06b7b8e5554c7",
          "message": "feat(map): assemble Doom map records into a validated Map graph (#155) (#205)",
          "timestamp": "2026-07-09T18:29:45-04:00",
          "tree_id": "cbd0f05084939525aa552b1be7047c8912ec5e4e",
          "url": "https://github.com/masriamir/crustywad/commit/2b634eea78652706ead1e7ff34a06b7b8e5554c7"
        },
        "date": 1783636699788,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 398,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4432,
            "range": "± 65",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37112,
            "range": "± 852",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 406,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4491,
            "range": "± 185",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37705,
            "range": "± 813",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27973,
            "range": "± 431",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27943,
            "range": "± 102",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20140,
            "range": "± 120",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20125,
            "range": "± 129",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15959,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1067,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11312,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17764,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 30679,
            "range": "± 82",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4707,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10723,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4716,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21096,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 25988,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 448,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16247,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 683556,
            "range": "± 26731",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 431,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14702,
            "range": "± 91",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 687930,
            "range": "± 20517",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 996,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 34375,
            "range": "± 159",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1484,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 46850,
            "range": "± 415",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2040097,
            "range": "± 40582",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "3880264ffdd9b96745eb0107c8e51c3bd4e9e6a0",
          "message": "test(fuzz): codify the O(input) allocation invariant; add ADR-0016 hardening checklist (#195) (#206)",
          "timestamp": "2026-07-09T23:09:42-04:00",
          "tree_id": "bbb877e8e18e8022601cf1c3942f6c54f5bc4f5d",
          "url": "https://github.com/masriamir/crustywad/commit/3880264ffdd9b96745eb0107c8e51c3bd4e9e6a0"
        },
        "date": 1783653481399,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 309,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 2931,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 29101,
            "range": "± 954",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 316,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 2964,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 29296,
            "range": "± 1259",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 22204,
            "range": "± 104",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 22163,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 16934,
            "range": "± 326",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 16883,
            "range": "± 132",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 32,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 11977,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 836,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 9473,
            "range": "± 128",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 14636,
            "range": "± 148",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 25670,
            "range": "± 120",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4130,
            "range": "± 156",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 9169,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4156,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 18062,
            "range": "± 219",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 18476,
            "range": "± 140",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 342,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 10912,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 707344,
            "range": "± 10022",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 351,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 12557,
            "range": "± 204",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 692598,
            "range": "± 27268",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 736,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 24165,
            "range": "± 1808",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1136,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 37647,
            "range": "± 1208",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2236693,
            "range": "± 130834",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "301005823+crustywad-release[bot]@users.noreply.github.com",
            "name": "crustywad-release[bot]",
            "username": "crustywad-release[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "37906496ff1d8ecc0051f465f1550f063cce4e64",
          "message": "chore: release (#192)",
          "timestamp": "2026-07-10T00:19:34-04:00",
          "tree_id": "a7bf260bb1a113b45784a2b1cce21b989151f8ae",
          "url": "https://github.com/masriamir/crustywad/commit/37906496ff1d8ecc0051f465f1550f063cce4e64"
        },
        "date": 1783657688391,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 400,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4349,
            "range": "± 206",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37589,
            "range": "± 1060",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 406,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4453,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37957,
            "range": "± 879",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28308,
            "range": "± 344",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28150,
            "range": "± 200",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21551,
            "range": "± 299",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21649,
            "range": "± 464",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18179,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1098,
            "range": "± 82",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10527,
            "range": "± 257",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17806,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 30688,
            "range": "± 80",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4675,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11208,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4720,
            "range": "± 263",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20836,
            "range": "± 464",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 26100,
            "range": "± 102",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 443,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 18084,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 727489,
            "range": "± 19997",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 435,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 18505,
            "range": "± 91",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 720178,
            "range": "± 20484",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1007,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 35034,
            "range": "± 416",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1544,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 52622,
            "range": "± 979",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 1983309,
            "range": "± 71897",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "bbd07fd03c33317e9ba2ef8f9ca4f369e3409a39",
          "message": "test(map): Heretic and Doom II map support via the Doom path (#56) (#208)",
          "timestamp": "2026-07-10T15:11:32-04:00",
          "tree_id": "8a809751c6790d9fa1beceb894ce795f2585f57a",
          "url": "https://github.com/masriamir/crustywad/commit/bbd07fd03c33317e9ba2ef8f9ca4f369e3409a39"
        },
        "date": 1783711205725,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 397,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4278,
            "range": "± 85",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38864,
            "range": "± 668",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 409,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4470,
            "range": "± 123",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 40008,
            "range": "± 1588",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28635,
            "range": "± 373",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28492,
            "range": "± 740",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20745,
            "range": "± 190",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20800,
            "range": "± 227",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 17171,
            "range": "± 563",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1073,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10591,
            "range": "± 148",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17200,
            "range": "± 858",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 35611,
            "range": "± 109",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4752,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10970,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4696,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21836,
            "range": "± 476",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 21908,
            "range": "± 129",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 441,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16384,
            "range": "± 136",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 799638,
            "range": "± 24110",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 457,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 19464,
            "range": "± 421",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 801396,
            "range": "± 23744",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1010,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 39180,
            "range": "± 303",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1551,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 54084,
            "range": "± 1579",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2264950,
            "range": "± 52981",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7e9b6c6a6468af82f78fbea373bb3f2ca743b3d2",
          "message": "docs(adr): add ADR-0017 UDMF representation and parsing strategy (#57) (#209)",
          "timestamp": "2026-07-10T15:22:09-04:00",
          "tree_id": "28719dbf4ef43743ba4b654af345e5ef6ebf2153",
          "url": "https://github.com/masriamir/crustywad/commit/7e9b6c6a6468af82f78fbea373bb3f2ca743b3d2"
        },
        "date": 1783711878610,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 381,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3821,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38472,
            "range": "± 1129",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 391,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3856,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 39275,
            "range": "± 893",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28645,
            "range": "± 121",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28594,
            "range": "± 2650",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22905,
            "range": "± 471",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22851,
            "range": "± 820",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15110,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1112,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11013,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19367,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 32649,
            "range": "± 195",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5300,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11801,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5322,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22948,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23667,
            "range": "± 104",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 434,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 15293,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 792778,
            "range": "± 15596",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 429,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14351,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 812860,
            "range": "± 41824",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 957,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 31531,
            "range": "± 85",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1480,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 47861,
            "range": "± 316",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2289212,
            "range": "± 20195",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "41caa77a51bc2410906fb54a9d9f1ec7c5b07d67",
          "message": "docs: define epic aggregate Status convention on the project board (#217)",
          "timestamp": "2026-07-10T17:34:04-04:00",
          "tree_id": "7acbea43e5ab6cd928efbcb466df6d3567f508d8",
          "url": "https://github.com/masriamir/crustywad/commit/41caa77a51bc2410906fb54a9d9f1ec7c5b07d67"
        },
        "date": 1783719861974,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 380,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4444,
            "range": "± 166",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 41897,
            "range": "± 1217",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 401,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4481,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 42272,
            "range": "± 616",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 36635,
            "range": "± 169",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 36199,
            "range": "± 159",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 23481,
            "range": "± 144",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 23514,
            "range": "± 748",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18024,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1278,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11077,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19043,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33095,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5348,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11838,
            "range": "± 65",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5348,
            "range": "± 116",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22917,
            "range": "± 76",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23930,
            "range": "± 80",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 421,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 14548,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 723454,
            "range": "± 12045",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 424,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14871,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 712967,
            "range": "± 14522",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1010,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 31461,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1488,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 47790,
            "range": "± 180",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2401330,
            "range": "± 17103",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f9e0505a04ef216989a400e51715bd26e3c7b780",
          "message": "docs: gate PR readiness on green CI + toolchain parity (#219)",
          "timestamp": "2026-07-10T19:10:26-04:00",
          "tree_id": "1e5dbb806e4ff5116deb3890bf8df6178ba52a17",
          "url": "https://github.com/masriamir/crustywad/commit/f9e0505a04ef216989a400e51715bd26e3c7b780"
        },
        "date": 1783725648628,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 397,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4318,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38740,
            "range": "± 2931",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 406,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4464,
            "range": "± 70",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 39022,
            "range": "± 664",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27726,
            "range": "± 688",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28443,
            "range": "± 187",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21559,
            "range": "± 180",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21703,
            "range": "± 128",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18554,
            "range": "± 299",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1082,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10662,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17218,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33480,
            "range": "± 128",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4750,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11023,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4774,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21111,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23200,
            "range": "± 129",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 484,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 18398,
            "range": "± 326",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 862100,
            "range": "± 34295",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 461,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 20142,
            "range": "± 388",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 851077,
            "range": "± 32367",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1011,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 37366,
            "range": "± 171",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1530,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 53315,
            "range": "± 1736",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2690804,
            "range": "± 47509",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e244b4094935116ad3e6828440124cb4a25775c2",
          "message": "test(fixtures): generalize the local-fixture harness for Hexen & Doom 64 (#216) (#218)",
          "timestamp": "2026-07-10T19:27:38-04:00",
          "tree_id": "537bc18ddf82716377aa6f1f73490020d6ec0e04",
          "url": "https://github.com/masriamir/crustywad/commit/e244b4094935116ad3e6828440124cb4a25775c2"
        },
        "date": 1783726667185,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 398,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4322,
            "range": "± 214",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37082,
            "range": "± 706",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 408,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4482,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38015,
            "range": "± 970",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27576,
            "range": "± 641",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27485,
            "range": "± 508",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20316,
            "range": "± 126",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20237,
            "range": "± 123",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18395,
            "range": "± 81",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1062,
            "range": "± 77",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10486,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19317,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 31344,
            "range": "± 239",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4709,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10683,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4718,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20846,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23880,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 447,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 18384,
            "range": "± 331",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 786231,
            "range": "± 56453",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 427,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 17092,
            "range": "± 384",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 801319,
            "range": "± 18920",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 958,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 35928,
            "range": "± 238",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1500,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 50804,
            "range": "± 1380",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2339675,
            "range": "± 286391",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "401fe4bfa03f47331fedf190692c575320be8282",
          "message": "feat(map): Hexen map format support + MapFormat substrate (#55) (#221)",
          "timestamp": "2026-07-11T00:58:14-04:00",
          "tree_id": "f99cb1a1634725d0676c2b726d25d85c5528a782",
          "url": "https://github.com/masriamir/crustywad/commit/401fe4bfa03f47331fedf190692c575320be8282"
        },
        "date": 1783746403701,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 396,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4361,
            "range": "± 182",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37997,
            "range": "± 1030",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 406,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4309,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37977,
            "range": "± 623",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27713,
            "range": "± 377",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27605,
            "range": "± 245",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21590,
            "range": "± 182",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21511,
            "range": "± 94",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 16306,
            "range": "± 117",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1059,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10578,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17200,
            "range": "± 77",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33423,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4666,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10972,
            "range": "± 196",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4716,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21159,
            "range": "± 408",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23254,
            "range": "± 1609",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 428,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16827,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 727687,
            "range": "± 25941",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 434,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 17178,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 723760,
            "range": "± 19509",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 939,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 35923,
            "range": "± 286",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1500,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 46058,
            "range": "± 567",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2126948,
            "range": "± 33354",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b3c837f52f38ac39667b68bb5d73deb6e206a41b",
          "message": "refactor(map)!: reconcile graph types with ADR-0017 §1 (Special rename; MapThing id/height) (#222) (#223)",
          "timestamp": "2026-07-11T11:51:04-04:00",
          "tree_id": "256a2f79eb5fbaa61f8a00aceab2687d6b606787",
          "url": "https://github.com/masriamir/crustywad/commit/b3c837f52f38ac39667b68bb5d73deb6e206a41b"
        },
        "date": 1783785582339,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 395,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4357,
            "range": "± 461",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37187,
            "range": "± 709",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 399,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4412,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37499,
            "range": "± 610",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27795,
            "range": "± 424",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27611,
            "range": "± 222",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21680,
            "range": "± 162",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21710,
            "range": "± 372",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 40,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15423,
            "range": "± 133",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1052,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10492,
            "range": "± 151",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17794,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 30693,
            "range": "± 163",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4717,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10670,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4705,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20825,
            "range": "± 124",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 25871,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 432,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16650,
            "range": "± 65",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 726767,
            "range": "± 25507",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 429,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 17457,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 720472,
            "range": "± 24991",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 991,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 34777,
            "range": "± 200",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1533,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 50744,
            "range": "± 4667",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2117086,
            "range": "± 58959",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b9a4fe09de0562c0d78ecfcf7d5a943d2f393309",
          "message": "feat(map)!: UDMF foundation — Limits/ParseOptions.limits + resolve_* i32 widening (#58) (#224)",
          "timestamp": "2026-07-11T14:31:36-04:00",
          "tree_id": "b9516615cacce8dc9bfc3859b6f843c268a0d0f6",
          "url": "https://github.com/masriamir/crustywad/commit/b9a4fe09de0562c0d78ecfcf7d5a943d2f393309"
        },
        "date": 1783795236420,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 370,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4153,
            "range": "± 98",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37451,
            "range": "± 804",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 390,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4205,
            "range": "± 286",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37328,
            "range": "± 1089",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28284,
            "range": "± 489",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28191,
            "range": "± 429",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21500,
            "range": "± 212",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21402,
            "range": "± 209",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18363,
            "range": "± 151",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1105,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10504,
            "range": "± 116",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19305,
            "range": "± 86",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 31781,
            "range": "± 140",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4719,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10653,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4709,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20799,
            "range": "± 61",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23834,
            "range": "± 236",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 428,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16192,
            "range": "± 375",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 828045,
            "range": "± 23963",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 441,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 16200,
            "range": "± 204",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 833405,
            "range": "± 26542",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 995,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 38071,
            "range": "± 308",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1474,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 48052,
            "range": "± 1495",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2412734,
            "range": "± 117213",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "07a6e946f7786dd536070b45941dfb08c0a3aa68",
          "message": "docs: add milestone closeout (propose-and-confirm) workflow to CLAUDE.md (#225)",
          "timestamp": "2026-07-11T15:17:16-04:00",
          "tree_id": "4d07a5054266524961770d62be2adccddceadc76",
          "url": "https://github.com/masriamir/crustywad/commit/07a6e946f7786dd536070b45941dfb08c0a3aa68"
        },
        "date": 1783797980992,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 371,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3816,
            "range": "± 88",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37366,
            "range": "± 1141",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 389,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3887,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38115,
            "range": "± 1218",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28402,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28520,
            "range": "± 439",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21886,
            "range": "± 82",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21949,
            "range": "± 197",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 40,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15822,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1117,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11019,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19338,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 32603,
            "range": "± 71",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5330,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11782,
            "range": "± 146",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5325,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22798,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23689,
            "range": "± 97",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 413,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 14487,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 792310,
            "range": "± 43298",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 411,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 20807,
            "range": "± 189",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 775588,
            "range": "± 19399",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 961,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 36181,
            "range": "± 169",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1478,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 47047,
            "range": "± 552",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2415519,
            "range": "± 63726",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "936983c1d7671990e35b46a562e19eb5985d0480",
          "message": "chore: gitignore untracked local artifacts (root WADs + fuzz lock) (#226)",
          "timestamp": "2026-07-11T20:23:16-04:00",
          "tree_id": "cde5a93cf1aca24734139c2d88517c7dd14abb1e",
          "url": "https://github.com/masriamir/crustywad/commit/936983c1d7671990e35b46a562e19eb5985d0480"
        },
        "date": 1783816282564,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 342,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3921,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 36746,
            "range": "± 867",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 395,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3936,
            "range": "± 119",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 36550,
            "range": "± 918",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 17100,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 17115,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 14022,
            "range": "± 190",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 13982,
            "range": "± 179",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 12962,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1268,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10313,
            "range": "± 125",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 15683,
            "range": "± 149",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 27296,
            "range": "± 98",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 3560,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10510,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 3568,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20106,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 19638,
            "range": "± 95",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 376,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 11940,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 1312778,
            "range": "± 44490",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 391,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 11993,
            "range": "± 206",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 1464342,
            "range": "± 110127",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1030,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 31907,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1577,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 51376,
            "range": "± 2527",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 4186946,
            "range": "± 213934",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f6232679256618329174e3ea7d67d816d31a7eb7",
          "message": "feat(map): UDMF text-map parser (parse_udmf + map::udmf) — PR A of #58 (#227)",
          "timestamp": "2026-07-11T21:32:56-04:00",
          "tree_id": "8fceed17e2a6cefc5b624fea1a85447fd5776115",
          "url": "https://github.com/masriamir/crustywad/commit/f6232679256618329174e3ea7d67d816d31a7eb7"
        },
        "date": 1783820507657,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 403,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4562,
            "range": "± 135",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38608,
            "range": "± 2232",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 409,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4557,
            "range": "± 286",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 39294,
            "range": "± 1318",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28516,
            "range": "± 562",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28376,
            "range": "± 191",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22198,
            "range": "± 216",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22541,
            "range": "± 784",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18652,
            "range": "± 528",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1081,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10496,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17745,
            "range": "± 126",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 31058,
            "range": "± 132",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4676,
            "range": "± 117",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10678,
            "range": "± 413",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4719,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20840,
            "range": "± 474",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 25860,
            "range": "± 102",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 430,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 18046,
            "range": "± 86",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 735548,
            "range": "± 30153",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 421,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 16771,
            "range": "± 325",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 780363,
            "range": "± 35855",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 995,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 39169,
            "range": "± 1390",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1512,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 54629,
            "range": "± 1097",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2132275,
            "range": "± 99422",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "805f1772eef1f1f5e295759b83dab5ffb0ce40f0",
          "message": "feat(map): assemble UDMF maps into the Map graph (MapFormat::Udmf) — PR B of #58 (#228)",
          "timestamp": "2026-07-12T00:04:34-04:00",
          "tree_id": "4095ae108e19d24f2690654a73abb9b428bc19f6",
          "url": "https://github.com/masriamir/crustywad/commit/805f1772eef1f1f5e295759b83dab5ffb0ce40f0"
        },
        "date": 1783829613552,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 372,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3755,
            "range": "± 271",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 36154,
            "range": "± 538",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 389,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3838,
            "range": "± 61",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 36330,
            "range": "± 992",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 29948,
            "range": "± 74",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 29902,
            "range": "± 92",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22549,
            "range": "± 130",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22482,
            "range": "± 148",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15379,
            "range": "± 99",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1103,
            "range": "± 65",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11366,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18839,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33084,
            "range": "± 110",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5304,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11789,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5326,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 23221,
            "range": "± 54",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23788,
            "range": "± 100",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 420,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 14434,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 663403,
            "range": "± 9836",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 408,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14816,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 669744,
            "range": "± 5240",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 916,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 31028,
            "range": "± 405",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1453,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 46536,
            "range": "± 165",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2358076,
            "range": "± 10452",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "589015505cede53dbf61bb438e158d561421b34e",
          "message": "docs: ADR-0018 — Doom 64 map format (nested-WAD, raw records) (#229)",
          "timestamp": "2026-07-12T19:52:58-04:00",
          "tree_id": "0f3a63652ed3f6e9b9b8f0184419322574f76547",
          "url": "https://github.com/masriamir/crustywad/commit/589015505cede53dbf61bb438e158d561421b34e"
        },
        "date": 1783900926691,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 377,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4251,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 35677,
            "range": "± 994",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 390,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4289,
            "range": "± 138",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 36192,
            "range": "± 892",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28042,
            "range": "± 353",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28223,
            "range": "± 400",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21836,
            "range": "± 128",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21736,
            "range": "± 390",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 16077,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1108,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10587,
            "range": "± 61",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17223,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33568,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4717,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10988,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4710,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21135,
            "range": "± 162",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23142,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 427,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 18615,
            "range": "± 103",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 715054,
            "range": "± 17624",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 433,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14802,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 724879,
            "range": "± 14207",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 960,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 36221,
            "range": "± 138",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1531,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 57052,
            "range": "± 956",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2170602,
            "range": "± 81892",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "07751841b4725a54caa3c10a069a690fe3d6987e",
          "message": "feat(map): read Doom 64 nested-WAD maps into raw records (map::doom64) (#230)",
          "timestamp": "2026-07-12T22:19:51-04:00",
          "tree_id": "869df3cb44c4f0445539e6ba82e3caecd0ccc4e6",
          "url": "https://github.com/masriamir/crustywad/commit/07751841b4725a54caa3c10a069a690fe3d6987e"
        },
        "date": 1783909700707,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 328,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3785,
            "range": "± 127",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37689,
            "range": "± 609",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 404,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3774,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38076,
            "range": "± 717",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 19209,
            "range": "± 470",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 19308,
            "range": "± 361",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 15089,
            "range": "± 260",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 15103,
            "range": "± 255",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 35,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15113,
            "range": "± 199",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1293,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10179,
            "range": "± 162",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17129,
            "range": "± 335",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 30263,
            "range": "± 429",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 3199,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11015,
            "range": "± 160",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 3265,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21562,
            "range": "± 369",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 22084,
            "range": "± 324",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 406,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 13742,
            "range": "± 279",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 1195171,
            "range": "± 13289",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 395,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 13745,
            "range": "± 240",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 1165293,
            "range": "± 13990",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 994,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 38241,
            "range": "± 693",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1605,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 46053,
            "range": "± 2360",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2564753,
            "range": "± 39415",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "0c65c57d1369cc287660c7c8496623cfa0274b77",
          "message": "feat(map): serialize maps to UDMF TEXTMAP (write_udmf / add_udmf_map) (#231)",
          "timestamp": "2026-07-13T01:07:21-04:00",
          "tree_id": "0b45c8f7a02315bb7763cdd292c34c18f0859673",
          "url": "https://github.com/masriamir/crustywad/commit/0c65c57d1369cc287660c7c8496623cfa0274b77"
        },
        "date": 1783919765831,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 373,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3666,
            "range": "± 74",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37085,
            "range": "± 1015",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 390,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3712,
            "range": "± 128",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37903,
            "range": "± 1966",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28506,
            "range": "± 107",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28561,
            "range": "± 736",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22729,
            "range": "± 357",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22564,
            "range": "± 277",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 40,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15507,
            "range": "± 161",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1121,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11371,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18842,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33095,
            "range": "± 998",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5327,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11790,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5325,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 23351,
            "range": "± 81",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23661,
            "range": "± 709",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 320,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 13491,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 745053,
            "range": "± 43409",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 315,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 15345,
            "range": "± 150",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 813989,
            "range": "± 80697",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 876,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 30185,
            "range": "± 385",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1401,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 48043,
            "range": "± 2512",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2570234,
            "range": "± 115806",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1bcbd753dcc1a631cbd2a8d84220ff8c6cda8c33",
          "message": "docs: ADR-0019 — UDMF <-> Doom map format conversion (#232)",
          "timestamp": "2026-07-13T12:49:52-04:00",
          "tree_id": "a89c67bcc5bfd191e051970185d0b560e8c324a3",
          "url": "https://github.com/masriamir/crustywad/commit/1bcbd753dcc1a631cbd2a8d84220ff8c6cda8c33"
        },
        "date": 1783961906405,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 372,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3857,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37130,
            "range": "± 1064",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 390,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3848,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37388,
            "range": "± 718",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28366,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28388,
            "range": "± 133",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22626,
            "range": "± 127",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22538,
            "range": "± 145",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 40,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15776,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1095,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11369,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18848,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 32671,
            "range": "± 679",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5301,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11807,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5326,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 23128,
            "range": "± 104",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23509,
            "range": "± 110",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 321,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 15228,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 755702,
            "range": "± 22134",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 331,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 13367,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 769327,
            "range": "± 18774",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 831,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 30333,
            "range": "± 192",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1330,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 46404,
            "range": "± 1208",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2349434,
            "range": "± 23990",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cc755d4a7290e0b8ffa5111e4d69209e5673ed6a",
          "message": "feat(map): UDMF <-> Doom map conversion (library) (#233)",
          "timestamp": "2026-07-13T16:52:17-04:00",
          "tree_id": "038de83ddc7308789bb1f7fb7c8365c5f2e3a1eb",
          "url": "https://github.com/masriamir/crustywad/commit/cc755d4a7290e0b8ffa5111e4d69209e5673ed6a"
        },
        "date": 1783976471009,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 383,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3790,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37651,
            "range": "± 1169",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 391,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3833,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37916,
            "range": "± 1365",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27888,
            "range": "± 727",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27805,
            "range": "± 767",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21801,
            "range": "± 375",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21883,
            "range": "± 355",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15200,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1100,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11355,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18869,
            "range": "± 113",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33077,
            "range": "± 335",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5326,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11810,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5328,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 23366,
            "range": "± 117",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23605,
            "range": "± 148",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 408,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 13942,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 734341,
            "range": "± 30358",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 401,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14220,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 730001,
            "range": "± 19444",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 953,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 32717,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1491,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 49121,
            "range": "± 977",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2195284,
            "range": "± 16570",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 50801,
            "range": "± 145",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 51392,
            "range": "± 159",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cf0b66ebb9d7d259a1de0a7dfe78a9e3c88a858b",
          "message": "feat(cli): cwad convert — UDMF <-> Doom map conversion (#234)",
          "timestamp": "2026-07-13T18:31:35-04:00",
          "tree_id": "8643d40c84e3fa248e7e4573f21dc582a7323585",
          "url": "https://github.com/masriamir/crustywad/commit/cf0b66ebb9d7d259a1de0a7dfe78a9e3c88a858b"
        },
        "date": 1783982467160,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 400,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4384,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38365,
            "range": "± 1196",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 412,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4441,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 40032,
            "range": "± 699",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28410,
            "range": "± 419",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 29269,
            "range": "± 329",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21042,
            "range": "± 356",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20981,
            "range": "± 353",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18533,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1106,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10484,
            "range": "± 231",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17757,
            "range": "± 226",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 30639,
            "range": "± 123",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4718,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10662,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4706,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20750,
            "range": "± 819",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 26049,
            "range": "± 171",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 437,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 19034,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 990002,
            "range": "± 90327",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 425,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 20157,
            "range": "± 131",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 925324,
            "range": "± 44340",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1056,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 37488,
            "range": "± 808",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1508,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 50083,
            "range": "± 2736",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2756832,
            "range": "± 167876",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 51494,
            "range": "± 338",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 52120,
            "range": "± 687",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "80bed47f9c9c5c219d8cec787b0b27177d0cc4bd",
          "message": "docs(adr): mark ADR-0018 Accepted (#236)",
          "timestamp": "2026-07-13T18:40:39-04:00",
          "tree_id": "216f7a9c6c14265fbf40485db93a95361fd5c56e",
          "url": "https://github.com/masriamir/crustywad/commit/80bed47f9c9c5c219d8cec787b0b27177d0cc4bd"
        },
        "date": 1783983005528,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 402,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4404,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37973,
            "range": "± 770",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 415,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4551,
            "range": "± 118",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 39582,
            "range": "± 927",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27110,
            "range": "± 113",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27081,
            "range": "± 1102",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20913,
            "range": "± 197",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20893,
            "range": "± 145",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18913,
            "range": "± 335",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1109,
            "range": "± 412",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11590,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17248,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33483,
            "range": "± 88",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4717,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11032,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4671,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21135,
            "range": "± 274",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23159,
            "range": "± 774",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 442,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 18188,
            "range": "± 658",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 729575,
            "range": "± 20193",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 431,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14593,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 712630,
            "range": "± 21573",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 991,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 39801,
            "range": "± 1332",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1503,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 54275,
            "range": "± 1540",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 1922444,
            "range": "± 34550",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 51219,
            "range": "± 250",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 52269,
            "range": "± 515",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "301005823+crustywad-release[bot]@users.noreply.github.com",
            "name": "crustywad-release[bot]",
            "username": "crustywad-release[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2d7f1334aba56d90c1d2b7174b4783ae936581cf",
          "message": "chore: release (#215)",
          "timestamp": "2026-07-13T18:49:28-04:00",
          "tree_id": "c7fcb082c8a3b45057e98a48ac789876c3179082",
          "url": "https://github.com/masriamir/crustywad/commit/2d7f1334aba56d90c1d2b7174b4783ae936581cf"
        },
        "date": 1783983537494,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 399,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4254,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38115,
            "range": "± 807",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 409,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4475,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 39271,
            "range": "± 683",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 29706,
            "range": "± 4513",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 29986,
            "range": "± 174",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21133,
            "range": "± 319",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20916,
            "range": "± 187",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 16083,
            "range": "± 67",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1052,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10560,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17806,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 30683,
            "range": "± 116",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4684,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10685,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4744,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20876,
            "range": "± 81",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 25911,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 433,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 19140,
            "range": "± 65",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 708223,
            "range": "± 25962",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 434,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 17054,
            "range": "± 133",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 775150,
            "range": "± 31494",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1026,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 40506,
            "range": "± 136",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1485,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 54150,
            "range": "± 2174",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2140769,
            "range": "± 42179",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 52106,
            "range": "± 409",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 52389,
            "range": "± 322",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "37b5850cffb2bf96e13416bf341b812701549b6c",
          "message": "docs: pin documented crustywad version to 0.3 and guard against future drift (#237)",
          "timestamp": "2026-07-13T20:05:34-04:00",
          "tree_id": "eee4c63f97c0dc941d895af044f6060dedc3242d",
          "url": "https://github.com/masriamir/crustywad/commit/37b5850cffb2bf96e13416bf341b812701549b6c"
        },
        "date": 1783988040874,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 297,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3271,
            "range": "± 201",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 33196,
            "range": "± 624",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 341,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3256,
            "range": "± 118",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 33494,
            "range": "± 397",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 16473,
            "range": "± 267",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 16537,
            "range": "± 524",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 12633,
            "range": "± 348",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 12663,
            "range": "± 326",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 3,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 13004,
            "range": "± 208",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1105,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 8828,
            "range": "± 208",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 14386,
            "range": "± 217",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 20896,
            "range": "± 298",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 2834,
            "range": "± 118",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 9317,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 2733,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 18613,
            "range": "± 403",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 18691,
            "range": "± 350",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 350,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 12026,
            "range": "± 352",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 1065833,
            "range": "± 34368",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 356,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 11869,
            "range": "± 212",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 1050287,
            "range": "± 13394",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 930,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 33410,
            "range": "± 1415",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1388,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 40129,
            "range": "± 1736",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2255819,
            "range": "± 28197",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 39600,
            "range": "± 822",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 40397,
            "range": "± 690",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "301005823+crustywad-release[bot]@users.noreply.github.com",
            "name": "crustywad-release[bot]",
            "username": "crustywad-release[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "530a11910fee5fff705e2f6478797bbd20918683",
          "message": "chore: release (#238)",
          "timestamp": "2026-07-13T20:10:05-04:00",
          "tree_id": "512d125abe257646e9158625e87a59a3a2972f71",
          "url": "https://github.com/masriamir/crustywad/commit/530a11910fee5fff705e2f6478797bbd20918683"
        },
        "date": 1783988572878,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 400,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4297,
            "range": "± 70",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37922,
            "range": "± 1036",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 411,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4439,
            "range": "± 103",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38394,
            "range": "± 761",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27965,
            "range": "± 138",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 29087,
            "range": "± 326",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21247,
            "range": "± 165",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21159,
            "range": "± 268",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15407,
            "range": "± 92",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1064,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10488,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19337,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 31639,
            "range": "± 134",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4707,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10652,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4717,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20840,
            "range": "± 382",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23887,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 433,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16699,
            "range": "± 102",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 777389,
            "range": "± 22107",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 419,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 17201,
            "range": "± 113",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 794915,
            "range": "± 20644",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1045,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 36322,
            "range": "± 169",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1465,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 49343,
            "range": "± 625",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 1994895,
            "range": "± 76371",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 49933,
            "range": "± 3678",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 50991,
            "range": "± 321",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "60222f208e5edf4b43aa8c00a6fb2be6281c4347",
          "message": "chore: untrack the scripts __pycache__ artifact and ignore Python bytecode (#239)",
          "timestamp": "2026-07-13T21:59:35-04:00",
          "tree_id": "2b3e3eddca94600b08ca67634a14cdf5cb5e9578",
          "url": "https://github.com/masriamir/crustywad/commit/60222f208e5edf4b43aa8c00a6fb2be6281c4347"
        },
        "date": 1783994882091,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 398,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4140,
            "range": "± 217",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37984,
            "range": "± 812",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 414,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4174,
            "range": "± 204",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38744,
            "range": "± 914",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28659,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28607,
            "range": "± 217",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20994,
            "range": "± 563",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21025,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 44,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18408,
            "range": "± 203",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1070,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10750,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19353,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 31653,
            "range": "± 205",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4742,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11172,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4761,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20873,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23890,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 435,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 17773,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 710380,
            "range": "± 5606",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 430,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 19342,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 707443,
            "range": "± 14249",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1048,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 41732,
            "range": "± 258",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1476,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 53236,
            "range": "± 1943",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 1922023,
            "range": "± 34792",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 49969,
            "range": "± 222",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 51024,
            "range": "± 254",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c7eb8d32c36aa62a1d9d4e7190afe1e96f1dcd76",
          "message": "chore: gitignore the PWADS/ retail WAD collection (#259)",
          "timestamp": "2026-07-13T23:15:27-04:00",
          "tree_id": "5ae15d51f8c1371ce538122c18fb2cb2978927bd",
          "url": "https://github.com/masriamir/crustywad/commit/c7eb8d32c36aa62a1d9d4e7190afe1e96f1dcd76"
        },
        "date": 1783999473259,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 395,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4336,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37446,
            "range": "± 1079",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 407,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4424,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37558,
            "range": "± 717",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27383,
            "range": "± 655",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27222,
            "range": "± 513",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20552,
            "range": "± 234",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20557,
            "range": "± 146",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15936,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1050,
            "range": "± 54",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10531,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19357,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 31531,
            "range": "± 230",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4708,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10676,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4694,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20846,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23878,
            "range": "± 96",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 430,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 18332,
            "range": "± 109",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 687650,
            "range": "± 17424",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 425,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14635,
            "range": "± 70",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 655595,
            "range": "± 14662",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 990,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 40105,
            "range": "± 249",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1472,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 52827,
            "range": "± 1296",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 1924949,
            "range": "± 18949",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 50453,
            "range": "± 198",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 50564,
            "range": "± 127",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6ec5209c5626aec7f65c87077d099352a74c724e",
          "message": "fix(map)!: honor the 0xffff front-sidedef sentinel on both sides (ADR-0020) (#260)",
          "timestamp": "2026-07-14T00:22:47-04:00",
          "tree_id": "7a7af90104d127244c3cef75b8687e54aec14a9d",
          "url": "https://github.com/masriamir/crustywad/commit/6ec5209c5626aec7f65c87077d099352a74c724e"
        },
        "date": 1784003491673,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 399,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4364,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37983,
            "range": "± 582",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 409,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4473,
            "range": "± 161",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38465,
            "range": "± 612",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 26158,
            "range": "± 104",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27333,
            "range": "± 602",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20702,
            "range": "± 105",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20669,
            "range": "± 121",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15339,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1072,
            "range": "± 71",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10495,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19368,
            "range": "± 54",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 31640,
            "range": "± 85",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4675,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10654,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4706,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20844,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23870,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 433,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16619,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 741034,
            "range": "± 13505",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 448,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 13824,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 634729,
            "range": "± 28636",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1038,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 32536,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1467,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 50938,
            "range": "± 3168",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 1999568,
            "range": "± 47243",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 49803,
            "range": "± 131",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 50742,
            "range": "± 2459",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1fcd3a524115ae5d10bc6ec3ab3705e9e9e04898",
          "message": "test(sweep): add the env-gated retail-WAD sweep behind sweep-tests (#262)",
          "timestamp": "2026-07-14T00:46:47-04:00",
          "tree_id": "e9ff30381dfe64c1daf4246432b81a72e2aa0430",
          "url": "https://github.com/masriamir/crustywad/commit/1fcd3a524115ae5d10bc6ec3ab3705e9e9e04898"
        },
        "date": 1784004954200,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 380,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3908,
            "range": "± 115",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38972,
            "range": "± 918",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 391,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3832,
            "range": "± 74",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 39539,
            "range": "± 763",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 34848,
            "range": "± 2775",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 34904,
            "range": "± 2194",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21806,
            "range": "± 277",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21747,
            "range": "± 131",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 23714,
            "range": "± 3033",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1070,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11017,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18970,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 32985,
            "range": "± 220",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5326,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11816,
            "range": "± 247",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5380,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22819,
            "range": "± 328",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 24005,
            "range": "± 143",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 412,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 14890,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 853845,
            "range": "± 41963",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 400,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 18326,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 806407,
            "range": "± 28738",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 976,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 30602,
            "range": "± 89",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1461,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 45339,
            "range": "± 880",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2467634,
            "range": "± 72379",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 50193,
            "range": "± 119",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 51059,
            "range": "± 163",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6bb9b073928e19f09463166f1f957a61c470e058",
          "message": "test(sweep): use ParseOptions::strict() explicitly; surface the read_dir error in skip notes (#263)",
          "timestamp": "2026-07-14T06:37:29-04:00",
          "tree_id": "ed119f7985d38f5964b1135371ab345393cd139a",
          "url": "https://github.com/masriamir/crustywad/commit/6bb9b073928e19f09463166f1f957a61c470e058"
        },
        "date": 1784026005916,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 422,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4339,
            "range": "± 575",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38334,
            "range": "± 915",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 434,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4439,
            "range": "± 320",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 39412,
            "range": "± 847",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27194,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 26953,
            "range": "± 195",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21662,
            "range": "± 350",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21516,
            "range": "± 276",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 45,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15579,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1074,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10507,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19332,
            "range": "± 120",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 31631,
            "range": "± 389",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4731,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10686,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4732,
            "range": "± 91",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20888,
            "range": "± 369",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23968,
            "range": "± 246",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 447,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16409,
            "range": "± 71",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 725925,
            "range": "± 8214",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 461,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 15106,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 701112,
            "range": "± 6742",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1052,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 37257,
            "range": "± 117",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1517,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 53244,
            "range": "± 1741",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2190484,
            "range": "± 43958",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 54422,
            "range": "± 1325",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 55150,
            "range": "± 1582",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "490d9d7ef0732f3682c9fa78bb94ef09103f2840",
          "message": "feat(map)!: Doom 64 graph normalization — MapFormat::Doom64, TextureRef, engine light table (ADR-0021) (#265)",
          "timestamp": "2026-07-14T14:21:02-04:00",
          "tree_id": "78d3f8e90c34f531ad40fceae105630d12603c9c",
          "url": "https://github.com/masriamir/crustywad/commit/490d9d7ef0732f3682c9fa78bb94ef09103f2840"
        },
        "date": 1784053822904,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 404,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4250,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38972,
            "range": "± 620",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 457,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4448,
            "range": "± 159",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38856,
            "range": "± 690",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27469,
            "range": "± 239",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27960,
            "range": "± 187",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20817,
            "range": "± 159",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20749,
            "range": "± 336",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18636,
            "range": "± 113",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1070,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10496,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18039,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 30663,
            "range": "± 203",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4731,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10888,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4727,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20841,
            "range": "± 165",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 25953,
            "range": "± 54",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 458,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 18079,
            "range": "± 201",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 775256,
            "range": "± 29220",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 439,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 16409,
            "range": "± 130",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 704159,
            "range": "± 17131",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1035,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 34725,
            "range": "± 1740",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1501,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 51296,
            "range": "± 356",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2051794,
            "range": "± 45785",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 51909,
            "range": "± 218",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 52818,
            "range": "± 172",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6622e1c8d8063cd6dd9326950e151da0c0753fc5",
          "message": "feat(cli): cwad info delegates map detection to Wad::map_groups (#266)",
          "timestamp": "2026-07-14T15:23:05-04:00",
          "tree_id": "bac55cd9e8ead87fd3c9e79d5525b1646f7cdab9",
          "url": "https://github.com/masriamir/crustywad/commit/6622e1c8d8063cd6dd9326950e151da0c0753fc5"
        },
        "date": 1784057531553,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 401,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4351,
            "range": "± 253",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38521,
            "range": "± 1768",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 453,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4505,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 39504,
            "range": "± 1657",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 26572,
            "range": "± 277",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28018,
            "range": "± 539",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21916,
            "range": "± 305",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21752,
            "range": "± 520",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 16251,
            "range": "± 236",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1110,
            "range": "± 86",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10595,
            "range": "± 158",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17208,
            "range": "± 341",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33495,
            "range": "± 104",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4689,
            "range": "± 113",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11270,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4700,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21150,
            "range": "± 225",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23221,
            "range": "± 651",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 430,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16367,
            "range": "± 131",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 816533,
            "range": "± 47860",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 424,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 18877,
            "range": "± 121",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 804357,
            "range": "± 43385",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1001,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 33053,
            "range": "± 613",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1485,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 47241,
            "range": "± 1090",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2256183,
            "range": "± 119877",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 53143,
            "range": "± 334",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 53891,
            "range": "± 269",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "405fa68a5033ae407eaca8957b800dd6dc3ec357",
          "message": "feat(cli): validate --deep assembles every map with per-map reporting (#267)",
          "timestamp": "2026-07-14T17:25:12-04:00",
          "tree_id": "14a2578c0953995d635672458d643dcb703efade",
          "url": "https://github.com/masriamir/crustywad/commit/405fa68a5033ae407eaca8957b800dd6dc3ec357"
        },
        "date": 1784064861638,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 392,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4060,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 39215,
            "range": "± 678",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 406,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4069,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 40490,
            "range": "± 1803",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28793,
            "range": "± 837",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28667,
            "range": "± 125",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22973,
            "range": "± 246",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 23070,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 14685,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1089,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11489,
            "range": "± 215",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18859,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 32689,
            "range": "± 179",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5346,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11814,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5302,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22996,
            "range": "± 494",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23514,
            "range": "± 284",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 411,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 15413,
            "range": "± 178",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 839842,
            "range": "± 54417",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 406,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14497,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 829522,
            "range": "± 22852",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 970,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 31646,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1433,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 50181,
            "range": "± 861",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2804372,
            "range": "± 98027",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 51553,
            "range": "± 1347",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 52048,
            "range": "± 131",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d1585c515db54b240fa622ec2b8b012bd0ba31b1",
          "message": "feat(map): BSP traversal — SEGS/SSECTORS/NODES onto the Map graph (#268)",
          "timestamp": "2026-07-14T23:43:10-04:00",
          "tree_id": "6b7c140ee94f3ef7cce78ef0669767f335a8eb43",
          "url": "https://github.com/masriamir/crustywad/commit/d1585c515db54b240fa622ec2b8b012bd0ba31b1"
        },
        "date": 1784087529148,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 372,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3682,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 36007,
            "range": "± 779",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 389,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3741,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37030,
            "range": "± 884",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28569,
            "range": "± 212",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28585,
            "range": "± 904",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22562,
            "range": "± 275",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22515,
            "range": "± 233",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 40,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15566,
            "range": "± 85",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1093,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11353,
            "range": "± 225",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18878,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 32625,
            "range": "± 125",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5325,
            "range": "± 123",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11802,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5323,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22991,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23505,
            "range": "± 67",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 325,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 14650,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 747963,
            "range": "± 9811",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 311,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14150,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 809693,
            "range": "± 51159",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 860,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 36787,
            "range": "± 700",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1346,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 48701,
            "range": "± 539",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2405088,
            "range": "± 29573",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 48735,
            "range": "± 1478",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 48758,
            "range": "± 1263",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "71f0da087dedcc9ee8010d2e4ebf61872e77f0c4",
          "message": "test(sweep): gate-expecting extended-node collection via CRUSTYWAD_SWEEP_EXTENDED_DIR (#270)",
          "timestamp": "2026-07-15T00:19:29-04:00",
          "tree_id": "65528c7af22a57668834ffbd0a9029dc30dada1a",
          "url": "https://github.com/masriamir/crustywad/commit/71f0da087dedcc9ee8010d2e4ebf61872e77f0c4"
        },
        "date": 1784089733686,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 372,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3734,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38067,
            "range": "± 2005",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 388,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3802,
            "range": "± 184",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37958,
            "range": "± 647",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28574,
            "range": "± 97",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28591,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22577,
            "range": "± 284",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22519,
            "range": "± 118",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 40,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15479,
            "range": "± 71",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1116,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11015,
            "range": "± 644",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19334,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 32633,
            "range": "± 476",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5327,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11807,
            "range": "± 218",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5324,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22875,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23839,
            "range": "± 139",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 325,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 14710,
            "range": "± 464",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 858375,
            "range": "± 68867",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 310,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 15314,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 742505,
            "range": "± 61060",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 900,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 30532,
            "range": "± 813",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1343,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 45998,
            "range": "± 671",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2557061,
            "range": "± 49216",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 47978,
            "range": "± 100",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 48998,
            "range": "± 156",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b014b4bf811a89d71a3162979b47c1e026f1f132",
          "message": "fix(cli): only print the --lenient hint for writer errors lenient mode can recover (#272)",
          "timestamp": "2026-07-15T14:47:37-04:00",
          "tree_id": "0d3d158aa0e741841d6244e72632fb3bd1139d27",
          "url": "https://github.com/masriamir/crustywad/commit/b014b4bf811a89d71a3162979b47c1e026f1f132"
        },
        "date": 1784141827473,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 372,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3642,
            "range": "± 80",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 35974,
            "range": "± 651",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 390,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3724,
            "range": "± 88",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 36329,
            "range": "± 784",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 29341,
            "range": "± 61",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 29181,
            "range": "± 103",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22484,
            "range": "± 167",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22466,
            "range": "± 195",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 16466,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1092,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11778,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18996,
            "range": "± 120",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33028,
            "range": "± 95",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5324,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 12649,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5325,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22794,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23864,
            "range": "± 65",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 320,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 14130,
            "range": "± 76",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 692458,
            "range": "± 13663",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 315,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 15509,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 703915,
            "range": "± 17122",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 892,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 30112,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1364,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 47186,
            "range": "± 333",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2381037,
            "range": "± 25743",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 48449,
            "range": "± 123",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 49246,
            "range": "± 202",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6146707ac904116fc92197319be1a2da8eded1cc",
          "message": "feat(map): parse REJECT and BLOCKMAP into typed structures (#274)",
          "timestamp": "2026-07-15T21:34:27-04:00",
          "tree_id": "563ba9f84b8684dad71b90f732fbebdd427cc2fc",
          "url": "https://github.com/masriamir/crustywad/commit/6146707ac904116fc92197319be1a2da8eded1cc"
        },
        "date": 1784166216905,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 384,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3768,
            "range": "± 184",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 36063,
            "range": "± 490",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 390,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3928,
            "range": "± 70",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 36660,
            "range": "± 1102",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27852,
            "range": "± 120",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28885,
            "range": "± 164",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21717,
            "range": "± 123",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21703,
            "range": "± 325",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15674,
            "range": "± 137",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1122,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 12136,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19386,
            "range": "± 160",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33454,
            "range": "± 235",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5329,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 12464,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5337,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 23831,
            "range": "± 134",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 24098,
            "range": "± 115",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 424,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 15337,
            "range": "± 100",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 715448,
            "range": "± 8389",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 418,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14722,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 718893,
            "range": "± 3980",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 984,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 32342,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1455,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 48764,
            "range": "± 796",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2528142,
            "range": "± 21530",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 51506,
            "range": "± 140",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 52380,
            "range": "± 85",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "22b7f29e93ceb6847fc8ba5b457b932787fa19ab",
          "message": "feat(map): decode Doom 64 LEAFS render leaves onto the Map graph (#275)",
          "timestamp": "2026-07-15T23:36:03-04:00",
          "tree_id": "82a5a813b04b4f0fb9347865de48beee5c614930",
          "url": "https://github.com/masriamir/crustywad/commit/22b7f29e93ceb6847fc8ba5b457b932787fa19ab"
        },
        "date": 1784173498482,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 397,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4366,
            "range": "± 206",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38559,
            "range": "± 1138",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 407,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4445,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 39694,
            "range": "± 2271",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 29292,
            "range": "± 207",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 29257,
            "range": "± 447",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20735,
            "range": "± 188",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20723,
            "range": "± 264",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15836,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1088,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10575,
            "range": "± 82",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17186,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33450,
            "range": "± 1753",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4646,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10990,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4724,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21194,
            "range": "± 294",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23275,
            "range": "± 1422",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 431,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 18073,
            "range": "± 364",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 812704,
            "range": "± 76173",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 426,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14632,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 789552,
            "range": "± 19029",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1017,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 36780,
            "range": "± 243",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1536,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 56954,
            "range": "± 1797",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2247150,
            "range": "± 64913",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 48866,
            "range": "± 103",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 49337,
            "range": "± 156",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4f7c5a41d7b2000f1b951f97f2f0faea70bf315c",
          "message": "feat(map): decode Doom 64 MACROS scripts onto the Map graph (#276)",
          "timestamp": "2026-07-16T09:06:34-04:00",
          "tree_id": "274b90629872649f363ea801c4c07b2f936ee7f4",
          "url": "https://github.com/masriamir/crustywad/commit/4f7c5a41d7b2000f1b951f97f2f0faea70bf315c"
        },
        "date": 1784207924762,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 392,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4322,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37240,
            "range": "± 1146",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 404,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4379,
            "range": "± 103",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37284,
            "range": "± 4551",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28103,
            "range": "± 281",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28214,
            "range": "± 126",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21516,
            "range": "± 204",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21538,
            "range": "± 329",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 40,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15932,
            "range": "± 98",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1049,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10550,
            "range": "± 116",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19337,
            "range": "± 82",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 31719,
            "range": "± 286",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4697,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10690,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4602,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20865,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23837,
            "range": "± 365",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 432,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16332,
            "range": "± 101",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 705660,
            "range": "± 12892",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 436,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 20465,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 704613,
            "range": "± 22857",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1007,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 35977,
            "range": "± 609",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1526,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 53721,
            "range": "± 2376",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2207270,
            "range": "± 68195",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 51351,
            "range": "± 432",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 52369,
            "range": "± 122",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "ba7dba7f834f223c5fd9e89ed816cc2433175cfa",
          "message": "chore(release): enable release-plz semver_check and document breaking-commit marking (#277)",
          "timestamp": "2026-07-16T09:56:59-04:00",
          "tree_id": "7f6a0df17906b83a8e35e6fa48e9cfdb5ed1eadf",
          "url": "https://github.com/masriamir/crustywad/commit/ba7dba7f834f223c5fd9e89ed816cc2433175cfa"
        },
        "date": 1784210760666,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 380,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3907,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37062,
            "range": "± 844",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 392,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3929,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37688,
            "range": "± 1639",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28751,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28889,
            "range": "± 65",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22496,
            "range": "± 222",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22348,
            "range": "± 2536",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 42,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15629,
            "range": "± 474",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1074,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11356,
            "range": "± 147",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18874,
            "range": "± 89",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33034,
            "range": "± 210",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5325,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11782,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5302,
            "range": "± 74",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 23234,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23817,
            "range": "± 126",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 412,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 15391,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 683453,
            "range": "± 8097",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 405,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14557,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 730703,
            "range": "± 24514",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1000,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 32518,
            "range": "± 3453",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1434,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 45949,
            "range": "± 1659",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2383558,
            "range": "± 26558",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 50555,
            "range": "± 774",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 51626,
            "range": "± 353",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "301005823+crustywad-release[bot]@users.noreply.github.com",
            "name": "crustywad-release[bot]",
            "username": "crustywad-release[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "352b73342a6814db4ded574e671cf8f150cf3d28",
          "message": "chore: release (#278)",
          "timestamp": "2026-07-16T11:02:55-04:00",
          "tree_id": "e52dca7dacbaa338c84926268e0c1ee7e3b0ea30",
          "url": "https://github.com/masriamir/crustywad/commit/352b73342a6814db4ded574e671cf8f150cf3d28"
        },
        "date": 1784214750736,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 392,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4396,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37461,
            "range": "± 499",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 404,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4430,
            "range": "± 81",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38112,
            "range": "± 709",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27831,
            "range": "± 457",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27677,
            "range": "± 152",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21897,
            "range": "± 235",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22138,
            "range": "± 169",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 40,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18809,
            "range": "± 135",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1079,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10535,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17756,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 30777,
            "range": "± 222",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4717,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10652,
            "range": "± 259",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4661,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20846,
            "range": "± 1116",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 25980,
            "range": "± 441",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 464,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16139,
            "range": "± 450",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 765612,
            "range": "± 54455",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 451,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 16575,
            "range": "± 670",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 765455,
            "range": "± 39303",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1059,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 36485,
            "range": "± 388",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1486,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 49791,
            "range": "± 953",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2317054,
            "range": "± 70078",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 50976,
            "range": "± 87",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 52391,
            "range": "± 325",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a86607399217d1aad490000dc147aa444af2bf28",
          "message": "chore: rename retail WAD collection dirs to RETAIL/RETAIL-EXT (#279)",
          "timestamp": "2026-07-16T13:02:08-04:00",
          "tree_id": "62a3c0f140b15d3c90040e9bb73cf75a49758ee3",
          "url": "https://github.com/masriamir/crustywad/commit/a86607399217d1aad490000dc147aa444af2bf28"
        },
        "date": 1784221835396,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 302,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3319,
            "range": "± 252",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 32587,
            "range": "± 947",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 346,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3238,
            "range": "± 131",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 32897,
            "range": "± 1079",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 16865,
            "range": "± 662",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 16793,
            "range": "± 480",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 12241,
            "range": "± 355",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 12537,
            "range": "± 694",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 12988,
            "range": "± 235",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1139,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 8680,
            "range": "± 238",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 14975,
            "range": "± 563",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 20950,
            "range": "± 919",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 2741,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 9073,
            "range": "± 326",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 2765,
            "range": "± 109",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 18610,
            "range": "± 626",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 18667,
            "range": "± 626",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 359,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 12275,
            "range": "± 340",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 1066731,
            "range": "± 44056",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 354,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 12023,
            "range": "± 227",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 1067841,
            "range": "± 27738",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 899,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 31993,
            "range": "± 815",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1401,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 38944,
            "range": "± 2151",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2249020,
            "range": "± 27893",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 39785,
            "range": "± 1462",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 40572,
            "range": "± 870",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "fbec8f7308a848d8617b0da0593e649d4888bbc3",
          "message": "docs(adr): ADR-0022 graphics & texture layer landscape (#283)",
          "timestamp": "2026-07-16T13:43:46-04:00",
          "tree_id": "4780185c9b8eeee5ecbd11abf0c5fc8623e99278",
          "url": "https://github.com/masriamir/crustywad/commit/fbec8f7308a848d8617b0da0593e649d4888bbc3"
        },
        "date": 1784224392304,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 382,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3761,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38234,
            "range": "± 1118",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 392,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3746,
            "range": "± 67",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 36802,
            "range": "± 875",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28425,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28386,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22590,
            "range": "± 130",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22654,
            "range": "± 141",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15297,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1086,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11378,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18920,
            "range": "± 148",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 32630,
            "range": "± 137",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5336,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11802,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5328,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 23005,
            "range": "± 117",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23545,
            "range": "± 608",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 427,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 15167,
            "range": "± 394",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 756636,
            "range": "± 10918",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 415,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14758,
            "range": "± 117",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 752070,
            "range": "± 12634",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 960,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 37085,
            "range": "± 303",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1454,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 49448,
            "range": "± 933",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2597289,
            "range": "± 67943",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 52186,
            "range": "± 117",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 52545,
            "range": "± 112",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e1be76c276286eea4a6284c2c1622b44932404bd",
          "message": "feat(wad): marker-delimited directory section API (#284)",
          "timestamp": "2026-07-16T21:04:30-04:00",
          "tree_id": "9d51e94a5bfd55205410389f18315c1987b497cf",
          "url": "https://github.com/masriamir/crustywad/commit/e1be76c276286eea4a6284c2c1622b44932404bd"
        },
        "date": 1784250814476,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 287,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 2978,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 28813,
            "range": "± 551",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 300,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 2929,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 29132,
            "range": "± 1132",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 21652,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 21547,
            "range": "± 263",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 17169,
            "range": "± 282",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 17147,
            "range": "± 218",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 3,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 32,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 11808,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 841,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 9048,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 14597,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 25619,
            "range": "± 234",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4130,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 9164,
            "range": "± 228",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4129,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 18124,
            "range": "± 310",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 18428,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 259,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 10779,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 610758,
            "range": "± 43793",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 246,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 10382,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 587829,
            "range": "± 18947",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 697,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 24859,
            "range": "± 145",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1039,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 37027,
            "range": "± 279",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2127476,
            "range": "± 32064",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 39972,
            "range": "± 338",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 40687,
            "range": "± 92",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "ffdc3db6919b97bd1e1d7d6dd072558768e5b1f2",
          "message": "chore(fuzz): require eight leading hex digits in the corpus ignore glob (#285)",
          "timestamp": "2026-07-16T22:36:30-04:00",
          "tree_id": "207e9a5888dfc3dcec91ef825b7dd8b7bc66183e",
          "url": "https://github.com/masriamir/crustywad/commit/ffdc3db6919b97bd1e1d7d6dd072558768e5b1f2"
        },
        "date": 1784256349691,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 373,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3695,
            "range": "± 105",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 36262,
            "range": "± 788",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 391,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3888,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 36495,
            "range": "± 1370",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28649,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28750,
            "range": "± 132",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21685,
            "range": "± 156",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21590,
            "range": "± 134",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 40,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15479,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1090,
            "range": "± 283",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11371,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18870,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 32827,
            "range": "± 964",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5330,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11815,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5327,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22990,
            "range": "± 260",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23541,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 320,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 14127,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 714382,
            "range": "± 4429",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 316,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14098,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 712002,
            "range": "± 8088",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 843,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 30291,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1314,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 46981,
            "range": "± 191",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2498308,
            "range": "± 10945",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 51469,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 52280,
            "range": "± 179",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "05eb8131ad4042f53e215807fed60b629a249070",
          "message": "feat(map): Doom 64 texture-name resolution and convert-gate lift (#287)",
          "timestamp": "2026-07-17T09:38:53-04:00",
          "tree_id": "4f010223e48d9cea09de55987d0cad242eee509c",
          "url": "https://github.com/masriamir/crustywad/commit/05eb8131ad4042f53e215807fed60b629a249070"
        },
        "date": 1784296097136,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 381,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3788,
            "range": "± 462",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 36413,
            "range": "± 891",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 392,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3858,
            "range": "± 82",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 36865,
            "range": "± 1078",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 29185,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 29093,
            "range": "± 89",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22691,
            "range": "± 172",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22667,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 40,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15238,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1084,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11014,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19367,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 32628,
            "range": "± 317",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5322,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11803,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5325,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 23017,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23601,
            "range": "± 121",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 323,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 14025,
            "range": "± 95",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 765492,
            "range": "± 24552",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 319,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 15414,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 755734,
            "range": "± 36806",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 899,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 30978,
            "range": "± 181",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1336,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 44816,
            "range": "± 1337",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2114655,
            "range": "± 67276",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 51147,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 51955,
            "range": "± 97",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "422a6c4abd0596a8cb0ab06e1a57587d5def1b4e",
          "message": "feat(gfx): classic graphics decode — pictures, flats, PLAYPAL/COLORMAP (#293)",
          "timestamp": "2026-07-17T15:10:34-04:00",
          "tree_id": "bc5e15f86c2118c06c42d6b48574c985815e9154",
          "url": "https://github.com/masriamir/crustywad/commit/422a6c4abd0596a8cb0ab06e1a57587d5def1b4e"
        },
        "date": 1784316003358,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 373,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3785,
            "range": "± 88",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37557,
            "range": "± 2016",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 389,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3767,
            "range": "± 171",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38748,
            "range": "± 1056",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 34702,
            "range": "± 704",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 34240,
            "range": "± 1888",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22743,
            "range": "± 183",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22745,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 14679,
            "range": "± 207",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1076,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11013,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19004,
            "range": "± 270",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33038,
            "range": "± 319",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5325,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11813,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5328,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22794,
            "range": "± 384",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23825,
            "range": "± 1007",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 331,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 13954,
            "range": "± 223",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 849925,
            "range": "± 32566",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 317,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 13914,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 824782,
            "range": "± 22540",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 876,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 32342,
            "range": "± 188",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1353,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 46304,
            "range": "± 1332",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2389726,
            "range": "± 78789",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 45581,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 46252,
            "range": "± 152",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "49699333+dependabot[bot]@users.noreply.github.com",
            "name": "dependabot[bot]",
            "username": "dependabot[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d50eefad4c838020f012d7213ea794b785691f92",
          "message": "chore(deps): bump clap from 4.6.1 to 4.6.2 (#289)",
          "timestamp": "2026-07-17T15:29:31-04:00",
          "tree_id": "73b1e0a48a05bf3c845fbb36d566425426883209",
          "url": "https://github.com/masriamir/crustywad/commit/d50eefad4c838020f012d7213ea794b785691f92"
        },
        "date": 1784317147456,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 399,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4615,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38575,
            "range": "± 553",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 408,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4545,
            "range": "± 173",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38781,
            "range": "± 1331",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 29230,
            "range": "± 151",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 29188,
            "range": "± 105",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20573,
            "range": "± 76",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20457,
            "range": "± 821",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18771,
            "range": "± 269",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1073,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10575,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17200,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 35509,
            "range": "± 71",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4679,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10982,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4719,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21666,
            "range": "± 104",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 21490,
            "range": "± 103",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 320,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 17101,
            "range": "± 80",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 746130,
            "range": "± 27694",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 312,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 15723,
            "range": "± 138",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 817762,
            "range": "± 25034",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 914,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 35101,
            "range": "± 144",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1373,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 51088,
            "range": "± 437",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2159271,
            "range": "± 149710",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 44210,
            "range": "± 384",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 45158,
            "range": "± 281",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "49699333+dependabot[bot]@users.noreply.github.com",
            "name": "dependabot[bot]",
            "username": "dependabot[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6066aedd676c0ce105e2d8abf46b2ce37d8d6a13",
          "message": "chore(deps): bump release-plz/action from 0.5.130 to 0.5.131 (#288)",
          "timestamp": "2026-07-17T15:52:42-04:00",
          "tree_id": "fdef153c12465043fcd9f533f9a03cc2446fb828",
          "url": "https://github.com/masriamir/crustywad/commit/6066aedd676c0ce105e2d8abf46b2ce37d8d6a13"
        },
        "date": 1784318481687,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 358,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3698,
            "range": "± 167",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 36669,
            "range": "± 1127",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 386,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3773,
            "range": "± 110",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 36958,
            "range": "± 948",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 18508,
            "range": "± 219",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 18500,
            "range": "± 333",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 14707,
            "range": "± 241",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 14479,
            "range": "± 330",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 35,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 34,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 14324,
            "range": "± 376",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1189,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 9626,
            "range": "± 327",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 16228,
            "range": "± 439",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 27659,
            "range": "± 829",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 2904,
            "range": "± 86",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10183,
            "range": "± 268",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 2917,
            "range": "± 176",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20513,
            "range": "± 608",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 21218,
            "range": "± 665",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 258,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 12137,
            "range": "± 357",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 1128069,
            "range": "± 26257",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 258,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 11972,
            "range": "± 352",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 1136863,
            "range": "± 29586",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 885,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 34840,
            "range": "± 1066",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1382,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 39476,
            "range": "± 2707",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2491210,
            "range": "± 47456",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 38605,
            "range": "± 923",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 39330,
            "range": "± 972",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9279dc627e1cb217cdaebfb3e0bdddfe11cc6e5c",
          "message": "ci: bump codeql-action to 4.37.1 in lockstep and group actions updates (#294)",
          "timestamp": "2026-07-17T16:06:17-04:00",
          "tree_id": "0372dae4b931e0004fb2a08de230f6b1fd427b90",
          "url": "https://github.com/masriamir/crustywad/commit/9279dc627e1cb217cdaebfb3e0bdddfe11cc6e5c"
        },
        "date": 1784319300297,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 403,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4050,
            "range": "± 181",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 39519,
            "range": "± 1001",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 415,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4479,
            "range": "± 170",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 39518,
            "range": "± 525",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27878,
            "range": "± 707",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 29608,
            "range": "± 722",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20816,
            "range": "± 141",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20817,
            "range": "± 209",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 14617,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1080,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10589,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17214,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 35517,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4622,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10987,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4721,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21648,
            "range": "± 77",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 21659,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 328,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 15383,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 874958,
            "range": "± 34731",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 319,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 16517,
            "range": "± 74",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 855184,
            "range": "± 37334",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 919,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 33886,
            "range": "± 191",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1373,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 45272,
            "range": "± 1232",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2411765,
            "range": "± 154217",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 46440,
            "range": "± 232",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 47413,
            "range": "± 708",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "318e720a1ee4929562cff377b5995dbb5f05e80e",
          "message": "feat(gfx)!: texture composition — PNAMES, TEXTUREx, and the R_GenerateComposite contract (#295)",
          "timestamp": "2026-07-17T20:45:21-04:00",
          "tree_id": "876b52eb5591d7a816d7276cf3c34cf671698b52",
          "url": "https://github.com/masriamir/crustywad/commit/318e720a1ee4929562cff377b5995dbb5f05e80e"
        },
        "date": 1784336033165,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 361,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4047,
            "range": "± 352",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 41398,
            "range": "± 1115",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 409,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4087,
            "range": "± 96",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 40975,
            "range": "± 723",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 20228,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 20302,
            "range": "± 290",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 14800,
            "range": "± 135",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 14744,
            "range": "± 132",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 3,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 13715,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1555,
            "range": "± 155",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10166,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 15955,
            "range": "± 985",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 26940,
            "range": "± 225",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 3562,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11064,
            "range": "± 361",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 3560,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 19244,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 19859,
            "range": "± 143",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 395,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 11920,
            "range": "± 81",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 1804898,
            "range": "± 131998",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 391,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 11950,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 1803962,
            "range": "± 131730",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1077,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 31846,
            "range": "± 158",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1618,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 52517,
            "range": "± 2726",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 5014626,
            "range": "± 183437",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 45880,
            "range": "± 99",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 46685,
            "range": "± 276",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cd1e1322b741695761c534a770f591d9a28aed27",
          "message": "feat(gfx): Doom 64 PNG decode behind the doom64-gfx feature (#298)",
          "timestamp": "2026-07-18T09:05:36-04:00",
          "tree_id": "326af50df1f3947a3ffe54eb75176640b5271b89",
          "url": "https://github.com/masriamir/crustywad/commit/cd1e1322b741695761c534a770f591d9a28aed27"
        },
        "date": 1784380504614,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 388,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3975,
            "range": "± 244",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 40297,
            "range": "± 862",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 393,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4032,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 42500,
            "range": "± 2418",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28592,
            "range": "± 148",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28565,
            "range": "± 154",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22870,
            "range": "± 301",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22792,
            "range": "± 228",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15779,
            "range": "± 1359",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1122,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11367,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18844,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 32642,
            "range": "± 143",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5329,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11785,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5328,
            "range": "± 128",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 23082,
            "range": "± 140",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23468,
            "range": "± 141",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 320,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 14123,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 746102,
            "range": "± 5858",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 326,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 13694,
            "range": "± 122",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 756469,
            "range": "± 2540",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 858,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 30383,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1310,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 45358,
            "range": "± 681",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2670942,
            "range": "± 26765",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 54839,
            "range": "± 743",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 55805,
            "range": "± 1887",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "301005823+crustywad-release[bot]@users.noreply.github.com",
            "name": "crustywad-release[bot]",
            "username": "crustywad-release[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "abd16b4f413b28452478982999d66f442d0116d0",
          "message": "chore: release (#286)",
          "timestamp": "2026-07-18T14:57:48-04:00",
          "tree_id": "d0b9c46500f7398bb513534775eb4a198736933c",
          "url": "https://github.com/masriamir/crustywad/commit/abd16b4f413b28452478982999d66f442d0116d0"
        },
        "date": 1784401580191,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 310,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3314,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 33405,
            "range": "± 624",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 351,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3360,
            "range": "± 100",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 33570,
            "range": "± 1012",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 17048,
            "range": "± 291",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 16994,
            "range": "± 280",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 13262,
            "range": "± 322",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 13122,
            "range": "± 340",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 3,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 13214,
            "range": "± 267",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1161,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 9158,
            "range": "± 216",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 14761,
            "range": "± 250",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 21355,
            "range": "± 446",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 2834,
            "range": "± 58",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 9531,
            "range": "± 212",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 2827,
            "range": "± 85",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 18993,
            "range": "± 504",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 19175,
            "range": "± 343",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 230,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 11290,
            "range": "± 181",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 1081857,
            "range": "± 11977",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 229,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 11220,
            "range": "± 794",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 1094802,
            "range": "± 15246",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 831,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 32017,
            "range": "± 639",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1275,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 39269,
            "range": "± 2120",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2361674,
            "range": "± 26094",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 40728,
            "range": "± 897",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 41120,
            "range": "± 1046",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1a433b35274c9a4bf7ecc7f203ef0b63d53d0d57",
          "message": "docs(adr): ADR-0023 audio layer — formats, staging, and Doom 64 containers (#300)",
          "timestamp": "2026-07-18T18:24:48-04:00",
          "tree_id": "37f04fa23af33b18708916e9d630361480671e2a",
          "url": "https://github.com/masriamir/crustywad/commit/1a433b35274c9a4bf7ecc7f203ef0b63d53d0d57"
        },
        "date": 1784414054795,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 398,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4310,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 39054,
            "range": "± 662",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 407,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4477,
            "range": "± 527",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 40295,
            "range": "± 915",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 29259,
            "range": "± 233",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 29268,
            "range": "± 204",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21405,
            "range": "± 147",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21522,
            "range": "± 167",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18132,
            "range": "± 116",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1084,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10502,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19321,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 31638,
            "range": "± 207",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4695,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10662,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4718,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 20802,
            "range": "± 137",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23802,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 335,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 17222,
            "range": "± 81",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 735752,
            "range": "± 15734",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 319,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 15880,
            "range": "± 345",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 761345,
            "range": "± 32019",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 931,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 34611,
            "range": "± 131",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1433,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 51431,
            "range": "± 371",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2225287,
            "range": "± 45708",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 53873,
            "range": "± 451",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 54717,
            "range": "± 232",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d6b284c9e381d5c05e3c2644e171e6ba3ffc1b18",
          "message": "feat(audio): AudioKind content detection plus DMX and PC-speaker sound decode (#305)",
          "timestamp": "2026-07-18T19:30:05-04:00",
          "tree_id": "08bacd3af2e7aef3d6f9077116680d14779a165b",
          "url": "https://github.com/masriamir/crustywad/commit/d6b284c9e381d5c05e3c2644e171e6ba3ffc1b18"
        },
        "date": 1784417978953,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 395,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4365,
            "range": "± 380",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38012,
            "range": "± 561",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 405,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4392,
            "range": "± 96",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38308,
            "range": "± 602",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 30105,
            "range": "± 1209",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 30116,
            "range": "± 250",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20588,
            "range": "± 137",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20566,
            "range": "± 209",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 18156,
            "range": "± 238",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1088,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10607,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17204,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33463,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4718,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10971,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4635,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21321,
            "range": "± 186",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23161,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 443,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 18087,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 741063,
            "range": "± 16855",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 443,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 16896,
            "range": "± 102",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 739386,
            "range": "± 17921",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1053,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 36516,
            "range": "± 267",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1542,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 53765,
            "range": "± 1624",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2103178,
            "range": "± 45310",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 54940,
            "range": "± 81",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 55176,
            "range": "± 382",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f7db3f4c873500f3421afff0218815d3a091350f",
          "message": "feat(audio): MUS score, MIDI/WAV chunk parsers, GENMIDI and DMXGUS instrument banks (#307)",
          "timestamp": "2026-07-18T22:56:19-04:00",
          "tree_id": "a03285e662c254679447acaa42aeaceb07b13d6e",
          "url": "https://github.com/masriamir/crustywad/commit/f7db3f4c873500f3421afff0218815d3a091350f"
        },
        "date": 1784430314726,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 389,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4325,
            "range": "± 69",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37702,
            "range": "± 1591",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 406,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4464,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38463,
            "range": "± 599",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28565,
            "range": "± 527",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27931,
            "range": "± 95",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 19965,
            "range": "± 178",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20068,
            "range": "± 621",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15404,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1054,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10575,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17195,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33429,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4717,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10964,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4663,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21073,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23114,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 434,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 19219,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 688303,
            "range": "± 17065",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 434,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 15015,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 652823,
            "range": "± 4020",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1060,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 36501,
            "range": "± 153",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1484,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 45972,
            "range": "± 273",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2081602,
            "range": "± 30151",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 47013,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 48288,
            "range": "± 232",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "67d8d6efe3b7f7da07430a1f39440ff487277020",
          "message": "feat(sections): recognize the KEX remaster's DM_START..DM_END music section (#308)",
          "timestamp": "2026-07-19T00:01:06-04:00",
          "tree_id": "879698fdc62dd9e76dccbc0b71cff3f9db588885",
          "url": "https://github.com/masriamir/crustywad/commit/67d8d6efe3b7f7da07430a1f39440ff487277020"
        },
        "date": 1784434233652,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 388,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4346,
            "range": "± 67",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 39509,
            "range": "± 1548",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 405,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4498,
            "range": "± 70",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 39374,
            "range": "± 647",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28182,
            "range": "± 160",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28198,
            "range": "± 221",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21869,
            "range": "± 342",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21860,
            "range": "± 596",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 16729,
            "range": "± 111",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1105,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10574,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17212,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 35496,
            "range": "± 269",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4723,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10996,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4633,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21792,
            "range": "± 527",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 21611,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 432,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16329,
            "range": "± 82",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 925116,
            "range": "± 27086",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 427,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 17664,
            "range": "± 920",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 897313,
            "range": "± 23605",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 991,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 36028,
            "range": "± 377",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1511,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 48749,
            "range": "± 631",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2628617,
            "range": "± 218969",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 47273,
            "range": "± 122",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 47905,
            "range": "± 710",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "01ccf0b71a0789cc0831c26642c02049af7b7f81",
          "message": "feat(audio): SNDINFO, SNDSEQ, and SNDCURVE script lumps (vanilla dialect) (#309)",
          "timestamp": "2026-07-19T01:08:53-04:00",
          "tree_id": "518c37c1b9ad38a248d9dd970c9c9dafb4d96c3c",
          "url": "https://github.com/masriamir/crustywad/commit/01ccf0b71a0789cc0831c26642c02049af7b7f81"
        },
        "date": 1784438286400,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 393,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4274,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37310,
            "range": "± 1057",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 402,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4398,
            "range": "± 206",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37350,
            "range": "± 537",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27943,
            "range": "± 100",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27888,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20326,
            "range": "± 149",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20281,
            "range": "± 895",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 16300,
            "range": "± 121",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1067,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10579,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17230,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 35834,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4717,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11034,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4719,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21821,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 21684,
            "range": "± 87",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 439,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 16107,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 702555,
            "range": "± 38346",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 442,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14415,
            "range": "± 91",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 709531,
            "range": "± 33492",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1024,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 37274,
            "range": "± 558",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1506,
            "range": "± 89",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 49052,
            "range": "± 323",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2058863,
            "range": "± 25875",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 52343,
            "range": "± 164",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 53305,
            "range": "± 242",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "80c5aeb452779d8c01421b8a31ed22f0be6cf331",
          "message": "feat(cli): audio-aware extract with WAV wrapping and MUS-to-MIDI conversion (#310)",
          "timestamp": "2026-07-19T10:32:58-04:00",
          "tree_id": "cf5c82c78c6bea1e7b60861a0c7b96b1b6907fc0",
          "url": "https://github.com/masriamir/crustywad/commit/80c5aeb452779d8c01421b8a31ed22f0be6cf331"
        },
        "date": 1784472110558,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 376,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4057,
            "range": "± 141",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 39188,
            "range": "± 918",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 414,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4079,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 40362,
            "range": "± 632",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 20755,
            "range": "± 124",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 20814,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 14664,
            "range": "± 94",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 14615,
            "range": "± 116",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 3,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 13837,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1274,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10471,
            "range": "± 171",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 15690,
            "range": "± 142",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 27582,
            "range": "± 675",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 3554,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10783,
            "range": "± 215",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 3540,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 19466,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 20160,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 391,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 12634,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 1222885,
            "range": "± 39013",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 383,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 12060,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 1341606,
            "range": "± 89030",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1033,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 35134,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1607,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 52962,
            "range": "± 2333",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 4408520,
            "range": "± 216948",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 47523,
            "range": "± 101",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 48122,
            "range": "± 76",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "5bd5d5f861c5c4d73257c0e1afa031d11b3e3b49",
          "message": "docs: milestones are scope-named; versions are tool-derived at ship time (#311)",
          "timestamp": "2026-07-19T12:15:50-04:00",
          "tree_id": "1b9c1a15c911571ccaf36f36d3cb7c651b4e2c1d",
          "url": "https://github.com/masriamir/crustywad/commit/5bd5d5f861c5c4d73257c0e1afa031d11b3e3b49"
        },
        "date": 1784478261985,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 392,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3943,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 41511,
            "range": "± 1970",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 400,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3992,
            "range": "± 99",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 42245,
            "range": "± 1383",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 34827,
            "range": "± 91",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 34803,
            "range": "± 2909",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21836,
            "range": "± 123",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21806,
            "range": "± 310",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 40,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15103,
            "range": "± 144",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1080,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11814,
            "range": "± 123",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18911,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33068,
            "range": "± 415",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5383,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 12310,
            "range": "± 77",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5322,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 23293,
            "range": "± 104",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23900,
            "range": "± 602",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 424,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 14620,
            "range": "± 122",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 827801,
            "range": "± 24592",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 420,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14374,
            "range": "± 399",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 899506,
            "range": "± 25151",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 954,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 32820,
            "range": "± 403",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1442,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 48309,
            "range": "± 1306",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2759213,
            "range": "± 163096",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 52502,
            "range": "± 612",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 53258,
            "range": "± 1156",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "be9695980270b4348a3171f36ea5fa64b78f6f8e",
          "message": "docs(adr): ADR-0024 — clean-room nodebuilder (BLOCKMAP, REJECT, classic BSP) (#312)",
          "timestamp": "2026-07-19T15:37:50-04:00",
          "tree_id": "1344da7025a3f770aae1fc7b0db62068b2577b39",
          "url": "https://github.com/masriamir/crustywad/commit/be9695980270b4348a3171f36ea5fa64b78f6f8e"
        },
        "date": 1784490436192,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 412,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 4617,
            "range": "± 100",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 39383,
            "range": "± 935",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 421,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 4699,
            "range": "± 210",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 41436,
            "range": "± 967",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 27988,
            "range": "± 169",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 27971,
            "range": "± 996",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 20777,
            "range": "± 141",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 20844,
            "range": "± 366",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15265,
            "range": "± 218",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1073,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10579,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17200,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33440,
            "range": "± 85",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 4717,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10968,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 4712,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 21085,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23197,
            "range": "± 404",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 437,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 18318,
            "range": "± 111",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 760909,
            "range": "± 17381",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 436,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 17057,
            "range": "± 208",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 747256,
            "range": "± 19870",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1037,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 35335,
            "range": "± 227",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1564,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 53042,
            "range": "± 1787",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2205221,
            "range": "± 34798",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 51967,
            "range": "± 157",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 52611,
            "range": "± 51",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d8bfda224d5dea02b393083b7fe277eba654a0f5",
          "message": "feat(build): nodebuild feature with BLOCKMAP and REJECT builders (ADR-0024 stage 1) (#317)",
          "timestamp": "2026-07-19T19:08:49-04:00",
          "tree_id": "21106b55710641a1f05b1ebe7b12014473ce3563",
          "url": "https://github.com/masriamir/crustywad/commit/d8bfda224d5dea02b393083b7fe277eba654a0f5"
        },
        "date": 1784503050377,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 237,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 2778,
            "range": "± 94",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 30748,
            "range": "± 895",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 276,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 2760,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 30669,
            "range": "± 536",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 13949,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 14076,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 11335,
            "range": "± 94",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 11325,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 34,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 38,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 11508,
            "range": "± 174",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1082,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 7544,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 12346,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 26291,
            "range": "± 240",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 2342,
            "range": "± 122",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 7979,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 2334,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 23223,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 16380,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 263,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 10355,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 1127532,
            "range": "± 2729",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 277,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 10303,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 1129004,
            "range": "± 7296",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 842,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 26475,
            "range": "± 2329",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1110,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 33319,
            "range": "± 1395",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2415758,
            "range": "± 62021",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 32861,
            "range": "± 316",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 32371,
            "range": "± 303",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2458bb2e026bd83bf4eefb4e6aa31802ef67e3bc",
          "message": "feat(build): the classic BSP pass (build_nodes) with the mixed-sector amendment (ADR-0024 stage 2) (#319)",
          "timestamp": "2026-07-20T12:00:33-04:00",
          "tree_id": "5cadcf0bf5881f03d9625d80d6fd20415ca36af0",
          "url": "https://github.com/masriamir/crustywad/commit/2458bb2e026bd83bf4eefb4e6aa31802ef67e3bc"
        },
        "date": 1784563808511,
        "tool": "cargo",
        "benches": [
          {
            "name": "parse/from_bytes_strict/small",
            "value": 379,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3825,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38760,
            "range": "± 965",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 394,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3972,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38944,
            "range": "± 2056",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28687,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28702,
            "range": "± 94",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 22592,
            "range": "± 117",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 22569,
            "range": "± 263",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 5,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 40,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 16183,
            "range": "± 103",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1095,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11368,
            "range": "± 120",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18834,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 33121,
            "range": "± 710",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5321,
            "range": "± 109",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11807,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5325,
            "range": "± 92",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 23378,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23601,
            "range": "± 294",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 411,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 15350,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 902997,
            "range": "± 36269",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 417,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14616,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 843544,
            "range": "± 61905",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 944,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 31344,
            "range": "± 101",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1442,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 50090,
            "range": "± 377",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2638053,
            "range": "± 194149",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 52100,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 52667,
            "range": "± 2143",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "87018def863bb930540860254cdb2898a2c30165",
          "message": "feat: nodebuilder stage 3 — add_doom_map_with_nodes + cwad convert --nodes (ADR-0024 §9.3) (#321)",
          "timestamp": "2026-07-20T17:02:49-04:00",
          "tree_id": "b4496413fc54d203fc7b220811e35e2b710824dc",
          "url": "https://github.com/masriamir/crustywad/commit/87018def863bb930540860254cdb2898a2c30165"
        },
        "date": 1784581939450,
        "tool": "cargo",
        "benches": [
          {
            "name": "build/nodes/build_nodes",
            "value": 369114,
            "range": "± 7174",
            "unit": "ns/iter"
          },
          {
            "name": "build/blockmap/build_blockmap",
            "value": 17295,
            "range": "± 115",
            "unit": "ns/iter"
          },
          {
            "name": "build/one_shot/add_doom_map_with_nodes",
            "value": 417566,
            "range": "± 4548",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/small",
            "value": 383,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3971,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38364,
            "range": "± 627",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 395,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3910,
            "range": "± 184",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 38617,
            "range": "± 3305",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28549,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28390,
            "range": "± 120",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21442,
            "range": "± 839",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21377,
            "range": "± 116",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 41,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15543,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1083,
            "range": "± 250",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11019,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 18999,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 32943,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5323,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11781,
            "range": "± 122",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5333,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22812,
            "range": "± 76",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23825,
            "range": "± 107",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 427,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 15082,
            "range": "± 157",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 829919,
            "range": "± 85103",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 462,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14873,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 775261,
            "range": "± 12039",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1016,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 33328,
            "range": "± 3046",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1458,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 53393,
            "range": "± 1181",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2721620,
            "range": "± 54860",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 51679,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 52375,
            "range": "± 118",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "301005823+crustywad-release[bot]@users.noreply.github.com",
            "name": "crustywad-release[bot]",
            "username": "crustywad-release[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "97dfe6ddb929dc078e5fbe3240d8b222c1b576df",
          "message": "chore: release (#318)",
          "timestamp": "2026-07-20T19:10:08-04:00",
          "tree_id": "e852a3b8dae89c9f79a752f60cc012300c674ae5",
          "url": "https://github.com/masriamir/crustywad/commit/97dfe6ddb929dc078e5fbe3240d8b222c1b576df"
        },
        "date": 1784589610546,
        "tool": "cargo",
        "benches": [
          {
            "name": "build/nodes/build_nodes",
            "value": 231452,
            "range": "± 4483",
            "unit": "ns/iter"
          },
          {
            "name": "build/blockmap/build_blockmap",
            "value": 15855,
            "range": "± 368",
            "unit": "ns/iter"
          },
          {
            "name": "build/one_shot/add_doom_map_with_nodes",
            "value": 278366,
            "range": "± 10381",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/small",
            "value": 343,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3788,
            "range": "± 98",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 38022,
            "range": "± 947",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 402,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3840,
            "range": "± 192",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 37523,
            "range": "± 1234",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 19811,
            "range": "± 339",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 19550,
            "range": "± 304",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 14029,
            "range": "± 285",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 14154,
            "range": "± 440",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 4,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 36,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15459,
            "range": "± 185",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1319,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 10143,
            "range": "± 126",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 17811,
            "range": "± 293",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 24695,
            "range": "± 322",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 3274,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 10713,
            "range": "± 128",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 3245,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22015,
            "range": "± 645",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 22219,
            "range": "± 334",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 424,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 14235,
            "range": "± 207",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 1245355,
            "range": "± 35932",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 423,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 14274,
            "range": "± 287",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 1228104,
            "range": "± 22463",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1105,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 38069,
            "range": "± 583",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1598,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 45356,
            "range": "± 1861",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2802446,
            "range": "± 289456",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 45871,
            "range": "± 854",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 47147,
            "range": "± 664",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "masriamir@users.noreply.github.com",
            "name": "Amir Masri",
            "username": "masriamir"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "30f5e290cdd5023e6598c906918a6fb03ce8a844",
          "message": "docs(adr): ADR-0025 — extended and GL node-format reading (#199) (#322)",
          "timestamp": "2026-07-20T20:24:13-04:00",
          "tree_id": "91c2eb4f5aa3930983e9ad180a8f5b5042071b52",
          "url": "https://github.com/masriamir/crustywad/commit/30f5e290cdd5023e6598c906918a6fb03ce8a844"
        },
        "date": 1784594033043,
        "tool": "cargo",
        "benches": [
          {
            "name": "build/nodes/build_nodes",
            "value": 368990,
            "range": "± 4061",
            "unit": "ns/iter"
          },
          {
            "name": "build/blockmap/build_blockmap",
            "value": 17364,
            "range": "± 102",
            "unit": "ns/iter"
          },
          {
            "name": "build/one_shot/add_doom_map_with_nodes",
            "value": 415137,
            "range": "± 15212",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/small",
            "value": 383,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/medium",
            "value": 3819,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_strict/large",
            "value": 37883,
            "range": "± 1286",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/small",
            "value": 394,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/medium",
            "value": 3790,
            "range": "± 113",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_bytes_lenient/large",
            "value": 36860,
            "range": "± 1163",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict",
            "value": 28587,
            "range": "± 569",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient",
            "value": 28654,
            "range": "± 580",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_strict_mmap",
            "value": 21800,
            "range": "± 516",
            "unit": "ns/iter"
          },
          {
            "name": "parse/from_path/medium_lenient_mmap",
            "value": 21627,
            "range": "± 134",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_index",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit",
            "value": 6,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_hit_last",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_by_name_miss",
            "value": 42,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_bytes",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lump_data",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/lumps_iter_count",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/clone",
            "value": 15751,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "lump_access/into_bytes",
            "value": 1098,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Thing_x1000",
            "value": 11796,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Linedef_x1000",
            "value": 19305,
            "range": "± 958",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sidedef_x1000",
            "value": 32636,
            "range": "± 170",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Vertex_x1000",
            "value": 5325,
            "range": "± 76",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Seg_x1000",
            "value": 11794,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Subsector_x1000",
            "value": 5321,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Node_x1000",
            "value": 22688,
            "range": "± 298",
            "unit": "ns/iter"
          },
          {
            "name": "map_records/Sector_x1000",
            "value": 23714,
            "range": "± 115",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/small",
            "value": 434,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/medium",
            "value": 14041,
            "range": "± 192",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_strict/large",
            "value": 766805,
            "range": "± 36279",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/small",
            "value": 423,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/medium",
            "value": 18989,
            "range": "± 110",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_lenient/large",
            "value": 723822,
            "range": "± 26864",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/10_lumps_256b",
            "value": 1042,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "write/build_from_scratch/100_lumps_4kib",
            "value": 33200,
            "range": "± 888",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/small",
            "value": 1447,
            "range": "± 67",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/medium",
            "value": 48190,
            "range": "± 443",
            "unit": "ns/iter"
          },
          {
            "name": "write/roundtrip/large",
            "value": 2360736,
            "range": "± 25277",
            "unit": "ns/iter"
          },
          {
            "name": "write/doom_map/write_doom_map",
            "value": 51783,
            "range": "± 109",
            "unit": "ns/iter"
          },
          {
            "name": "write/udmf_to_doom/write_doom_map",
            "value": 52127,
            "range": "± 99",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}