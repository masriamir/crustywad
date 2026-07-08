window.BENCHMARK_DATA = {
  "lastUpdate": 1783514837923,
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
      }
    ]
  }
}