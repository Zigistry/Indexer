#include <string>
#include <time.h>

std::string epoch_to_timestamp_string(const time_t epoch)
{
    char timestamp_string[21];
    struct tm tm { };

    gmtime_r(&epoch, &tm);

    strftime(timestamp_string, sizeof(timestamp_string),
        "%Y-%m-%dT%H:%M:%SZ", &tm);

    return timestamp_string;
}

time_t timestamp_to_epoch(const std::string& timestamp_string)
{
    struct tm tm { };
    strptime(timestamp_string.c_str(), "%Y-%m-%dT%H:%M:%SZ", &tm);
    return timegm(&tm);
}