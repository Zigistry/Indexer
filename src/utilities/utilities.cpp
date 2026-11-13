#include <iostream>

std::string epoch_to_timestamp_string(const time_t epoch)
{
    char timestamp_string[21];

    std::strftime(timestamp_string, sizeof(timestamp_string),
        "%Y-%m-%dT%H:%M:%SZ", std::gmtime(&epoch));
    return timestamp_string;
}
