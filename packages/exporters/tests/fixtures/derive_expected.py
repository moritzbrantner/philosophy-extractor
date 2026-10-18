#!/usr/bin/env python3
"""Independent reference-contract oracle; never calls the Rust exporter.

The source-span input is produced by the separately tested corpus-core intake.
Every native record id/revision/processor receipt stays verbatim; only envelope
ids, correlation families and the batch checksum are derived here.
"""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def digest(value):
    data = json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()
    return "sha256:" + hashlib.sha256(data).hexdigest()


def receipt(metadata, prefix):
    result = {"name": metadata[prefix + "Producer"], "revision": metadata[prefix + "ProducerVersion"]}
    assert result["revision"].startswith(("git:", "commit:", "sha256:"))
    if prefix + "ConfigHash" in metadata:
        result["configHash"] = metadata[prefix + "ConfigHash"]
    return result


def record(repository, scope, record_id, revision, evidence_class, producer, parents=None):
    result = {"class": evidence_class,
              "familyId": "family:source-record:" + digest([repository, scope, record_id]),
              "source": {"repository": repository, "scopeId": scope, "recordId": record_id, "revision": revision},
              "producer": producer}
    if parents:
        result["parentEvidenceIds"] = parents
    result["id"] = "evidence:philosophy-source:" + digest(result)
    return result


def main():
    data = json.loads((HERE / "document-source-spans.json").read_text())
    revision = (HERE / "exporter-revision.txt").read_text().strip()
    corpus = HERE.parents[2] / "source-ingestion/tests/fixtures/corpus_core/document-search-batch.jsonl"
    native = {line["record"]["record"]["id"]: line["record"]["record"]
              for line in map(json.loads, corpus.read_text().splitlines()) if "record" in line}
    repository = data["producer"]["name"]
    sources = {}
    evidence = []
    for source in data["sources"]:
        metadata = source["metadata"]
        representation = native[metadata["corpusCoreRepresentationId"]]
        assert metadata["corpusCoreRepresentationRevision"] == digest(representation)
        assert metadata["extractionProducer"] == representation["provenance"]["producer"]
        assert metadata["extractionProducerVersion"] == representation["provenance"]["producer_version"]
        assert metadata["extractionProducerInputs"] == representation["provenance"]["inputs"]
        scope = metadata.get("corpusCoreSourceId", source["id"])
        descriptor = record(repository, scope, metadata["corpusCoreRepresentationId"],
                            metadata["corpusCoreRepresentationRevision"], "metadata", receipt(metadata, "extraction"))
        sources[source["id"]] = (scope, descriptor)
        evidence.append(descriptor)
    for span in data["spans"]:
        segment = native[span["id"]]
        assert span["metadata"]["corpusCoreSegmentRevision"] == digest(segment)
        assert span["metadata"]["spanProducer"] == segment["provenance"]["producer"]
        assert span["metadata"]["spanProducerVersion"] == segment["provenance"]["producer_version"]
        assert span["metadata"]["spanProducerInputs"] == segment["provenance"]["inputs"]
        scope, parent = sources[span["sourceId"]]
        assert "sha256:" + hashlib.sha256(span["text"].encode()).hexdigest() == span["contentHash"]
        assert span["metadata"]["spanProducerInputs"] == [{"record_type": "representation", "id": parent["source"]["recordId"]}]
        evidence.append(record(repository, scope, span["id"], span["metadata"]["corpusCoreSegmentRevision"],
                               "source_text", receipt(span["metadata"], "span"), [parent["id"]]))
    evidence.sort(key=lambda item: item["id"])
    result = {"schema": "belief_evidence_interchange", "schemaVersion": 1,
              "exporter": {"name": "philosophy-extractor", "revision": revision},
              "revision": digest(evidence), "evidence": evidence}
    (HERE / "belief-evidence.json").write_text(json.dumps(result, sort_keys=True, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
