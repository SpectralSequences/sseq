import subprocess


def folder_pairs(folder):
    return (
        (int(split[0]), int(split[1]))
        for split in (f.stem.split("_") for f in folder.glob("*"))
    )


def test_secondary_job(tmp_path, monkeypatch):
    max_stems = 10
    goal_stem = 3
    monkeypatch.setenv("SECONDARY_JOB", str(goal_stem))
    subprocess.run(
        [
            "cargo",
            "run",
            "--example",
            "secondary",
            "--",
            "S_2",
            tmp_path,
            str(max_stems),
            "6",
        ],
        check=True,
    )
    for folder in [
        "augmentation_qis",
        "differentials",
        "kernels",
        "req_qis",
        "secondary_composites",
        "secondary_intermediates",
    ]:
        for s, t in folder_pairs(tmp_path / folder):
            assert t - s <= max_stems

    for s, _ in folder_pairs(tmp_path / "secondary_composites"):
        assert s == goal_stem

    for s, _ in folder_pairs(tmp_path / "secondary_intermediates"):
        assert s == goal_stem + 1
