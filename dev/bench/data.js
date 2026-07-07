window.BENCHMARK_DATA = {
  "lastUpdate": 1783394336795,
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
      }
    ]
  }
}