# Writes openvolley's view of every .dvw under <corpus>/dvw as one CSV per
# file in <out>, named after the path relative to <corpus> with "/" -> "__".
# Usage: Rscript oracle.R <corpus> <out>

suppressPackageStartupMessages(library(datavolley))

args <- commandArgs(trailingOnly = TRUE)
corpus <- normalizePath(args[1])
out_dir <- args[2]
dir.create(out_dir, showWarnings = FALSE, recursive = TRUE)

home <- paste0("home_p", 1:6)
visiting <- paste0("visiting_p", 1:6)
cols <- c("file_line_number", "code", "point_phase", "attack_phase",
          "start_coordinate", "mid_coordinate", "end_coordinate", "time",
          "video_file_number", "video_time", home, visiting)

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
