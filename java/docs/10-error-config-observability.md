# Error, config and observability

The error envelope is `{error:{code,message,requiredPermission,correlationId}}`. MicroProfile Config default ordinals: system properties 400 > env 300 > `microprofile-config.properties` 100. Health readiness/liveness, metrics and OTel traces carry the correlation id; logs carry tenant/slice/use case, never secrets.
