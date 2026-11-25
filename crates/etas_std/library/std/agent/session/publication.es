module std.agent.session;

import std.memory.{StorageContext, StorageError, StorageOperationRef, WriteOutcome, ReconcileResult};
import std.runtime.value.Data;
import std.crypto.Blake3;


type ContextPublication = { private wire: string };

impl SessionContextContent {
    flow encode(self, context: StorageContext) -> string ![Error<StorageError>] {
        if std.text.trim(self.text) == "" {
            perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "published context must not be empty", operation= None });
        }
        if self.provenance.length > context.max_nodes {
            perform Error.raise<StorageError>(StorageError { code= "limit_exceeded", message= "session mutation or receipt storage limit exceeded", operation= None });
        }
        var used = self.text.length.bits;
        if used > context.max_value_bytes.bits {
            perform Error.raise<StorageError>(StorageError { code= "limit_exceeded", message= "session mutation or receipt storage limit exceeded", operation= None });
        }
        var ordered: std.collections.OrderedMap<string, string> = std.collections.OrderedMap.new();
        for (key, value) in self.provenance {
            if key == "" {
                perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "context provenance key is empty", operation= None });
            }
            used = used + key.length.bits + value.length.bits;
            if used > context.max_value_bytes.bits {
                perform Error.raise<StorageError>(StorageError { code= "limit_exceeded", message= "session mutation or receipt storage limit exceeded", operation= None });
            }
            ordered = ordered.insert(key, value);
        }
        var provenance: Array<(string, Data)> = [];
        for (key, value) in ordered.iter() {
            provenance = provenance.push((key, Data.String(value)));
        }
        let content = Data.Record([("text", Data.String(self.text)), ("provenance", Data.Record(provenance))]);
        return match content.tagged(context.max_value_bytes) {
            Ok(text) => text,
            Err(error) => perform Error.raise<StorageError>(StorageError { code= error.code, message= error.message, operation= None }),
        };
    }
}

impl ContextPublication {
    flow new(config: SessionConfig, fence: SessionHistoryFence, content: SessionContextContent, context: StorageContext) -> ContextPublication ![Error<StorageError>] {
        let session = match Data.capture(config.id) {
            Ok(Data.String(value)) => value,
            Ok(_) => std.core.abort("SessionId representation invariant"),
            Err(error) => perform Error.raise<StorageError>(StorageError { code= error.code, message= error.message, operation= None }),
        };
        let subject = fence.subject(context);
        if session == "" || session != subject {
            perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "context fence belongs to another session", operation= None });
        }
        let content_bytes = content.encode(context);
        if session.length.bits + fence.opaque.length.bits + content_bytes.length.bits > context.max_value_bytes.bits {
            perform Error.raise<StorageError>(StorageError { code= "limit_exceeded", message= "session mutation or receipt storage limit exceeded", operation= None });
        }
        let request = Data.Record([
            ("session", Data.String(session)),
            ("fence", Data.String(fence.opaque)),
            ("content", Data.String(content_bytes)),
        ]);
        let wire = match request.tagged(context.max_value_bytes) {
            Ok(text) => text,
            Err(error) => perform Error.raise<StorageError>(StorageError { code= error.code, message= error.message, operation= None }),
        };
        return ContextPublication { wire= wire };
    }

    flow fingerprint(self) -> string ![] {
        let digest = Blake3.derive_key("etas.session.context-publication.v1", self.wire.utf8);
        var output = "";
        for byte in digest {
            let high = std.text.char_get("0123456789abcdef", byte.bits / 16).unwrap();
            let low = std.text.char_get("0123456789abcdef", byte.bits % 16).unwrap();
            output = output + std.text.from_char(high) + std.text.from_char(low);
        }
        return output;
    }
}

impl SessionConfig {
    public flow prepare_context(self, fence: SessionHistoryFence, content: SessionContextContent) -> StorageOperationRef ![Error<StorageError>] {
        let context = StorageContext.current();
        let request = ContextPublication.new(self, fence, content, context);
        let key = context.operation_key();
        return StorageOperationRef { key= key, fingerprint= request.fingerprint() };
    }

    public flow publish_context(self, fence: SessionHistoryFence, content: SessionContextContent, operation: StorageOperationRef) -> WriteOutcome<SessionContextReceipt, SessionContextRejection> ![Error<StorageError>] {
        operation.validate();
        let request = ContextPublication.new(self, fence, content, StorageContext.current());
        if request.fingerprint() != operation.fingerprint {
            perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "storage operation identity is bound to a different request", operation= None });
        }
        // This is the actual backend CAS: it authenticates the fence/seal,
        // generation and context version atomically and returns retained proof.
        return perform std.effects.Session.publish(self, fence, content, operation);
    }

    public flow reconcile_context(self, operation: StorageOperationRef) -> ReconcileResult<SessionContextReceipt, SessionContextRejection> ![Error<StorageError>] {
        operation.validate();
        // Query retained evidence with current read authority; do not retry an
        // uncertain mutation or infer non-commit from absent/expired evidence.
        return perform std.effects.Session.reconcile(self, operation);
    }
}
