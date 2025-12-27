#!/usr/bin/env bash
# Written in [Amber](https://amber-lang.com/)
# version: 0.5.1-alpha
# We cannot import `bash_version` from `env.ab` because it imports `text.ab` making a circular dependency.
# This is a workaround to avoid that issue and the import system should be improved in the future.
file_append__40_v0() {
    local path=$1
    local content=$2
    command_0="$(echo "${content}" >> "${path}")"
    __status=$?
    if [ "${__status}" != 0 ]; then
        ret_file_append40_v0=''
        return "${__status}"
    fi
    ret_file_append40_v0="${command_0}"
    return 0
}

usecases_3=("webhook-direct" "webhook-tgin" "webhook-tgin-3" "webhook-tgin-4" "webhook-tgin-5" "longpull-direct" "longpull-tgin" "longpull-tgin-3" "longpull-tgin-4" "longpull-tgin-5")
rps_values_4=(500 1000 2000 5000 8000 10000)
declare -r args_5=("$0" "$@")
make clean
__status=$?
if [ "${__status}" != 0 ]; then
    exit "${__status}"
fi
make build
__status=$?
if [ "${__status}" != 0 ]; then
    exit "${__status}"
fi
for rps_7 in "${rps_values_4[@]}"; do
    for mode_8 in "${usecases_3[@]}"; do
        command_4="$(make ${mode_8} RPS=${rps_7})"
        __status=$?
        if [ "${__status}" != 0 ]; then
            exit "${__status}"
        fi
        output_9="${command_4}"
        command_5="$(echo "${output_9}" | grep "Requests Sent:" | tr -s ' ' | xargs | cut -d ' ' -f 3)"
        __status=$?
        if [ "${__status}" != 0 ]; then
            exit "${__status}"
        fi
        sent_10="${command_5}"
        command_6="$(echo "${output_9}" | grep "Responses Recv:" | tr -s ' ' | xargs | cut -d ' ' -f 3)"
        __status=$?
        if [ "${__status}" != 0 ]; then
            exit "${__status}"
        fi
        recv_11="${command_6}"
        command_7="$(echo "${output_9}" | grep "Errors (Net):" | tr -s ' ' | xargs | cut -d ' ' -f 3)"
        __status=$?
        if [ "${__status}" != 0 ]; then
            exit "${__status}"
        fi
        errors_12="${command_7}"
        command_8="$(echo "${output_9}" | grep "Loss Rate:" | tr -s ' ' | xargs | cut -d ' ' -f 3 | tr -d '%')"
        __status=$?
        if [ "${__status}" != 0 ]; then
            exit "${__status}"
        fi
        loss_13="${command_8}"
        command_9="$(echo "${output_9}" | grep "Min:" | tr -s ' ' | xargs | cut -d ' ' -f 2)"
        __status=$?
        if [ "${__status}" != 0 ]; then
            exit "${__status}"
        fi
        min_14="${command_9}"
        command_10="$(echo "${output_9}" | grep "Mean:" | tr -s ' ' | xargs | cut -d ' ' -f 2)"
        __status=$?
        if [ "${__status}" != 0 ]; then
            exit "${__status}"
        fi
        mean_15="${command_10}"
        command_11="$(echo "${output_9}" | grep "p50:" | tr -s ' ' | xargs | cut -d ' ' -f 2)"
        __status=$?
        if [ "${__status}" != 0 ]; then
            exit "${__status}"
        fi
        p50_16="${command_11}"
        command_12="$(echo "${output_9}" | grep "p95:" | tr -s ' ' | xargs | cut -d ' ' -f 2)"
        __status=$?
        if [ "${__status}" != 0 ]; then
            exit "${__status}"
        fi
        p95_17="${command_12}"
        command_13="$(echo "${output_9}" | grep "p99:" | tr -s ' ' | xargs | cut -d ' ' -f 2)"
        __status=$?
        if [ "${__status}" != 0 ]; then
            exit "${__status}"
        fi
        p99_18="${command_13}"
        command_14="$(echo "${output_9}" | grep "Max:" | tr -s ' ' | xargs | cut -d ' ' -f 2)"
        __status=$?
        if [ "${__status}" != 0 ]; then
            exit "${__status}"
        fi
        max_19="${command_14}"
        make clean
        __status=$?
        if [ "${__status}" != 0 ]; then
            exit "${__status}"
        fi
        echo "RPS=${rps_7} -> Sent: ${sent_10}, Resv: ${recv_11}, Errors: ${errors_12}, Loss: ${loss_13}%,  Min: ${min_14}ms, Mean: ${mean_15}ms, P50: ${p50_16}ms, P95: ${p95_17}ms, P99: ${p99_18}ms, Max: ${max_19}ms"
        file_append__40_v0 "results.csv" "${mode_8},${rps_7},${sent_10},${recv_11},${errors_12},${loss_13},${min_14},${mean_15},${p50_16},${p95_17},${p99_18},${max_19}"
        __status=$?
        if [ "${__status}" != 0 ]; then
            exit "${__status}"
        fi
    done
done
