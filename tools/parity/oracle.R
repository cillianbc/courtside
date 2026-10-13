# Writes openvolley's view of every .dvw under <corpus>/dvw as one CSV per
# file in <out>, named after the path relative to <corpus> with "/" -> "__".
# Usage: Rscript oracle.R <corpus> <out>
#        Rscript oracle.R validate <dir> <out>
# The validate mode writes dv_validate's messages (validation level 3,
# default options) for every .dvw under <dir>, named the same way.

suppressPackageStartupMessages(library(datavolley))

args <- commandArgs(trailingOnly = TRUE)
if (length(args) == 3 && args[1] == "validate") {
    dir <- normalizePath(args[2])
    out_dir <- args[3]
    dir.create(out_dir, showWarnings = FALSE, recursive = TRUE)
    files <- list.files(dir, pattern = "\\.dvw$", recursive = TRUE,
                        ignore.case = TRUE, full.names = TRUE)
    failed <- 0L
    for (f in files) {
        rel <- sub(paste0("^", dir, "/"), "", normalizePath(f), fixed = FALSE)
        out <- file.path(out_dir, paste0(gsub("/", "__", rel, fixed = TRUE), ".csv"))
        res <- tryCatch({
            x <- dv_read(f, insert_technical_timeouts = FALSE, do_warn = FALSE)
            v <- dv_validate(x, validation_level = 3)
            v <- v[, c("file_line_number", "message"), drop = FALSE]
            write.csv(v, out, row.names = FALSE, na = "", fileEncoding = "UTF-8")
            sprintf("ok   %s (%d messages)", rel, nrow(v))
        }, error = function(e) {
            failed <<- failed + 1L
            sprintf("FAIL %s: %s", rel, conditionMessage(e))
        })
        cat(res, "\n")
    }
    cat(sprintf("datavolley %s dv_validate, %d files, %d failed\n",
                packageVersion("datavolley"), length(files), failed))
    quit(status = if (failed > 0) 1 else 0)
}
corpus <- normalizePath(args[1])
out_dir <- args[2]
dir.create(out_dir, showWarnings = FALSE, recursive = TRUE)

home <- paste0("home_p", 1:6)
visiting <- paste0("visiting_p", 1:6)
# Raw [3SCOUT] columns (dvw-codec parity), parsed code fields (vb-grammar
# parity), then derived match state (vb-engine parity).
cols <- c("file_line_number", "code", "point_phase", "attack_phase",
          "start_coordinate", "mid_coordinate", "end_coordinate", "time",
          "video_file_number", "video_time", home, visiting,
          "team", "player_number", "skill", "evaluation_code", "attack_code",
          "set_code", "set_type", "start_zone", "end_zone", "end_subzone",
          "custom_code", "timeout", "substitution", "point", "end_of_set",
          # Derived columns (vb-engine parity).
          "set_number", "home_team_score", "visiting_team_score",
          "home_setter_position", "visiting_setter_position", "point_id",
          "team_touch_id", "point_won_by", "serving_team", "phase",
          "winning_attack", "home_score_start_of_point",
          "visiting_score_start_of_point")

files <- list.files(file.path(corpus, "dvw"), pattern = "\\.dvw$", recursive = TRUE,
                    ignore.case = TRUE, full.names = TRUE)
failed <- 0L
for (f in files) {
    rel <- sub(paste0("^", corpus, "/"), "", normalizePath(f), fixed = FALSE)
    out <- file.path(out_dir, paste0(gsub("/", "__", rel, fixed = TRUE), ".csv"))
    res <- tryCatch({
        x <- dv_read(f, insert_technical_timeouts = FALSE)
        p <- x$plays
        p <- p[!is.na(p$file_line_number), , drop = FALSE]
        p$time <- ifelse(is.na(p$time), NA, format(p$time, "%H:%M:%S"))
        write.csv(p[, cols], out, row.names = FALSE, na = "", fileEncoding = "UTF-8")
        sprintf("ok   %s (%d rows)", rel, nrow(p))
    }, error = function(e) {
        failed <<- failed + 1L
        sprintf("FAIL %s: %s", rel, conditionMessage(e))
    })
    cat(res, "\n")
}
cat(sprintf("datavolley %s, %d files, %d failed\n", packageVersion("datavolley"), length(files), failed))
quit(status = if (failed > 0) 1 else 0)
