package com.example.app.widgets.presentation;

import com.example.app.shared.AppError;
import com.example.app.shared.Result;
import com.example.app.shared.TenantId;
import com.example.app.widgets.application.ArchiveWidget;
import com.example.app.widgets.domain.Widget;
import com.example.app.widgets.domain.WidgetId;
import jakarta.inject.Inject;
import jakarta.ws.rs.HeaderParam;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.PathParam;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.Context;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.core.SecurityContext;

@Path("/v1/app/widgets")
@Produces(MediaType.APPLICATION_JSON)
public final class WidgetResource {
    private final ArchiveWidget archiveWidget;

    @Inject
    public WidgetResource(ArchiveWidget archiveWidget) {
        this.archiveWidget = archiveWidget;
    }

    @POST
    @Path("/{id}/archive")
    public Response archive(
            @PathParam("id") String id,
            @HeaderParam("If-Match") String expected,
            @HeaderParam("Idempotency-Key") String idempotencyKey,
            @Context SecurityContext security) {
        // Replace with the validated JWT tenant claim in your product.
        TenantId tenant = new TenantId(security.getUserPrincipal() == null
                ? "public" : security.getUserPrincipal().getName());
        Result<Widget> result = archiveWidget.archive(
                tenant, new WidgetId(id), Long.parseLong(expected), idempotencyKey);
        if (result instanceof Result.Success<?> success) {
            return Response.ok(WidgetDto.from((Widget) success.value())).build();
        }
        Result.Failure<?> failure = (Result.Failure<?>) result;
        return toResponse(failure.error());
    }

    private Response toResponse(AppError error) {
        Response.Status status = switch (error.code()) {
            case "NOT_FOUND" -> Response.Status.NOT_FOUND;
            case "CONFLICT" -> Response.Status.CONFLICT;
            case "FORBIDDEN" -> Response.Status.FORBIDDEN;
            default -> Response.Status.INTERNAL_SERVER_ERROR;
        };
        return Response.status(status)
                .entity(new ErrorEnvelope(new ErrorBody(error.code(), error.message(),
                        error.requiredPermission(), error.correlationId())))
                .build();
    }

    record ErrorEnvelope(ErrorBody error) {
    }

    record ErrorBody(String code, String message, String requiredPermission, String correlationId) {
    }
}
