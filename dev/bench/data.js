window.BENCHMARK_DATA = {
  "lastUpdate": 1783816283870,
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
      }
    ]
  }
}