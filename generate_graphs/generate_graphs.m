clear all;
clc;
close all;



plot_avgs();
plot_percentiles();
plot_percentiles_tsl();
plot_percentiles_tsl_combined();
plot_tl("evgen", "Evgen");
plot_tl("revgen", "Evgen\_rust");
plot_tl("eventdev", "Eventdev\_pipeline");
plot_tl("reventdev", "Eventdev\_pipeline\_rust");
plot_tsl("evgen", 1, "Evgen");
plot_tsl("evgen", 2, "Evgen");
plot_tsl("evgen", 4, "Evgen");
plot_tsl("evgen", 8, "Evgen");
plot_tsl("evgen", 16, "Evgen");
plot_tsl("revgen", 1, "Evgen\_rust");
plot_tsl("revgen", 2, "Evgen\_rust");
plot_tsl("revgen", 4, "Evgen\_rust");
plot_tsl("revgen", 8, "Evgen\_rust");
plot_tsl("revgen", 16, "Evgen\_rust");
plot_tsl("eventdev", 1, "Eventdev\_pipeline");
plot_tsl("eventdev", 2, "Eventdev\_pipeline");
plot_tsl("eventdev", 4, "Eventdev\_pipeline");
plot_tsl("eventdev", 8, "Eventdev\_pipeline");
plot_tsl("eventdev", 16, "Eventdev\_pipeline");
plot_tsl("reventdev", 1, "Eventdev\_pipeline\_rust");
plot_tsl("reventdev", 2, "Eventdev\_pipeline\_rust");
plot_tsl("reventdev", 4, "Eventdev\_pipeline\_rust");
plot_tsl("reventdev", 8, "Eventdev\_pipeline\_rust");
plot_tsl("reventdev", 16, "Eventdev\_pipeline\_rust");


function B = get_cumulative_percentages(A)
    tmp = cumsum(A);
    B = tmp ./ tmp(end,:) * 100;
end

function A = get_percentile_array(L, percentile)
    [Rx,Cx] = find(L>=percentile);
    A = accumarray(Cx,Rx,[],@min);
end


function plot_percentiles()
    disp("Starting percentiles");
    P50 = zeros([28 4]);
    P99 = zeros([28 4]);

    i = 1;
    for program_name = ["evgen", "revgen", "eventdev", "reventdev"]
        disp("Percentiles for " + program_name);
        for num_cores = 1:28
            disp("Working on core " + num_cores);
            files = dir(strcat("data_", program_name, "_*_TL_", sprintf("%02d", num_cores),"_out.txt"));
            sum_latency = [];
            for file = files'
                data = importdata(file.name);
                new_length = max(length(sum_latency), length(data));

                sum_latency = [sum_latency; zeros(new_length - length(sum_latency), 1)] + [data; zeros(new_length - length(data), 1)];
            end
            sum_latency = get_cumulative_percentages(sum_latency);
            P50(num_cores, i) = get_percentile_array(sum_latency, 50);
            P99(num_cores, i) = get_percentile_array(sum_latency, 99);
        end
        i = i + 1;
    end

    Legend = ["Evgen - Median" "Evgen\_rust - Median" "Eventdev\_pipeline - Median" "Eventdev\_pipeline\_rust - Median", "Evgen - 99th percentile" "Evgen\_rust - 99th percentile" "Eventdev\_pipeline - 99th percentile" "Eventdev\_pipeline\_rust - 99th percentile"];

    fig = figure();
    styles = {'-','--', '-.'};
    colors = [0 0.4470  0.7410; 0.8500 0.3250 0.0980; 0.9290 0.6940 0.1250; 0.4940 0.1840 0.5560];
    ax = axes('XScale', 'linear', 'YScale', 'log');
    ax.ColorOrder = colors;
    ax.LineStyleOrder = styles;
    hold on;

    plot([P50 P99], "LineWidth", 1.5);
    legend(Legend, "Location", "southoutside", "NumColumns", 2);
    xlabel("Number of worker cores");
    ylabel("Latency (µs)");
    ylim([1 1000000000]);
    title("Median and 99th percentile tail latency")
    fig.Position = [1000 818 560 420];

    exportgraphics(fig, strcat("res_percentiles.pdf"),"ContentType","vector");
end


function plot_percentiles_tsl()
    disp("Starting TSL percentiles");

    for program_name = ["evgen", "revgen", "eventdev", "reventdev"]
        disp("Percentiles for " + program_name);
        P50 = zeros([28 3]);
        P99 = zeros([28 3]);
        for stage = 0:2
            for num_cores = 1:28
                disp("Working on core " + num_cores);
                files = dir(strcat("data_", program_name, "_*_TSL_", sprintf("%02d", num_cores),"_stage", string(stage), "_out.txt"));
                sum_latency = [];
                for file = files'
                    data = importdata(file.name);
                    new_length = max(length(sum_latency), length(data));

                    sum_latency = [sum_latency; zeros(new_length - length(sum_latency), 1)] + [data; zeros(new_length - length(data), 1)];
                end
                sum_latency = get_cumulative_percentages(sum_latency);
                P50(num_cores, stage + 1) = get_percentile_array(sum_latency, 50);
                P99(num_cores, stage + 1) = get_percentile_array(sum_latency, 99);
            end
        end

        fig = figure();
        colors = [0 0.4470  0.7410; 0.8500 0.3250 0.0980; 0.9290 0.6940 0.1250];
        styles = {'-','--', '-.'};
        ax = axes('XScale', 'linear', 'YScale', 'log');
        ax.ColorOrder = colors;
        ax.LineStyleOrder = styles;
        hold on;
    
        plot([P50 P99], "LineWidth", 1.5);
        legend({"Stage 1 - Median" "Stage 2 - Median" "Stage 3 - Median" "Stage 1 - 99th percentile" "Stage 2 - 99th percentile" "Stage 3 - 99th percentile"},  "Location", "southoutside", "NumColumns", 2);
        xlabel("Number of worker cores");
        ylabel("Latency (µs)");
        ylim([1 1000000000]);

        switch program_name
        case "evgen"
            title("Median and 99th percentile task swiching latency - Evgen")
        case "revgen"
            title("Median and 99th percentile task swiching latency - Evgen\_rust")
        case "eventdev"
            title("Median and 99th percentile task swiching latency - Eventdev\_pipeline")
        case "reventdev"
            title("Median and 99th percentile task swiching latency - Eventdev\_pipeline\_rust")
        end

        fig.Position = [1000 818 560 410];
    
        exportgraphics(fig, strcat("res_percentiles_tsl_", program_name, ".pdf"),"ContentType","vector");
        close all;
    end
end


function plot_percentiles_tsl_combined()
    disp("Starting TSL percentiles combined");

    for stage = 0:2
        disp("Percentiles for stage " + stage);
        P50 = zeros([28 4]);
        P99 = zeros([28 4]);
        i = 1;
        for program_name = ["evgen", "revgen", "eventdev", "reventdev"]
            for num_cores = 1:28
                disp("Working on " + program_name + ", core " + num_cores);
                files = dir(strcat("data_", program_name, "_*_TSL_", sprintf("%02d", num_cores),"_stage", string(stage), "_out.txt"));
                sum_latency = [];
                for file = files'
                    data = importdata(file.name);
                    new_length = max(length(sum_latency), length(data));

                    sum_latency = [sum_latency; zeros(new_length - length(sum_latency), 1)] + [data; zeros(new_length - length(data), 1)];
                end
                sum_latency = get_cumulative_percentages(sum_latency);
                P50(num_cores, i) = get_percentile_array(sum_latency, 50);
                P99(num_cores, i) = get_percentile_array(sum_latency, 99);
            end
            i = i + 1;
        end

        fig = figure();
        colors = [0 0.4470  0.7410; 0.8500 0.3250 0.0980; 0.9290 0.6940 0.1250; 0.4940 0.1840 0.5560];
        styles = {'-','--', '-.'};
        ax = axes('XScale', 'linear', 'YScale', 'log');
        ax.ColorOrder = colors;
        ax.LineStyleOrder = styles;
        hold on;

        plot([P50 P99], "LineWidth", 1.5);
        legend({"Evgen - Median" "Evgen\_rust - Median" "Eventdev\_pipeline - Median" "Eventdev\_pipeline\_rust - Median", "Evgen - 99th percentile" "Evgen\_rust - 99th percentile" "Eventdev\_pipeline - 99th percentile" "Eventdev\_pipeline\_rust - 99th percentile"},  "Location", "southoutside", "NumColumns", 2);
        xlabel("Number of worker cores");
        ylabel("Latency (µs)");
        ylim([1 1000000000]);

        title(strcat("Median and 99th percentile task swiching latency - Stage ", string(stage + 1)));

        fig.Position = [1000 818 560 420];

        exportgraphics(fig, strcat("res_percentiles_tsl_stage_", string(stage), ".pdf"),"ContentType","vector");
        close all;
    end
end

function plot_tl(program_name, display_name)
    disp("Starting TL for program " + program_name);

    fig = figure();
    styles = {'-','--', '-.'};
    colors = [0 0.4470  0.7410; 0.8500 0.3250 0.0980; 0.9290 0.6940 0.1250; 0.4940 0.1840 0.5560];
    ax = axes('XScale', 'log', 'YScale', 'linear');
    ax.LineStyleOrder = styles;
    ax.ColorOrder = colors;
    hold on;

    for num_cores = [1,2,3,4,6,8,10,12,16,22,28]
        disp("Working on core " + num_cores);
        files = dir(strcat("data_", program_name, "_*_TL_", sprintf("%02d", num_cores),"_out.txt"));
        sum_latency = [];
        for file = files'
            data = importdata(file.name);
            new_length = max(length(sum_latency), length(data));

            sum_latency = [sum_latency; zeros(new_length - length(sum_latency), 1)] + [data; zeros(new_length - length(data), 1)];
        end

        plot(get_cumulative_percentages(sum_latency), "LineWidth",1.5);
    end

    switch program_name
    case "evgen"
        legend({"1 core", "2 cores", "3 cores", "4 cores", "6 cores", "8 cores", "10 cores", "12 cores", "16 cores", "22 cores", "28 cores"}, "Location", "northeast");
    case "revgen"
        legend({"1 core", "2 cores", "3 cores", "4 cores", "6 cores", "8 cores", "10 cores", "12 cores", "16 cores", "22 cores", "28 cores"}, "Location", "northwest");
    case "eventdev"
        legend({"1 core", "2 cores", "3 cores", "4 cores", "6 cores", "8 cores", "10 cores", "12 cores", "16 cores", "22 cores", "28 cores"}, "Location", "northeast");
    case "reventdev"
        legend({"1 core", "2 cores", "3 cores", "4 cores", "6 cores", "8 cores", "10 cores", "12 cores", "16 cores", "22 cores", "28 cores"}, "Location", "northwest");
    end
    xlim([0 1000000000])
    xlabel("Latency (µs)");

    ylabel("Latency distribution");
    ytickformat('percentage');
    title(strcat("Total packet latency - ", display_name));
    fig.Position = [1000 818 560 390];

    exportgraphics(fig, strcat("res_tl_", program_name, ".pdf"),"ContentType","vector");
    close all;

end


function plot_tsl(program_name, num_cores, display_name)
    disp("Starting TSL for program " + program_name + " with " + num_cores + " cores");

    fig = figure();
    fig.Position = [2500 1500 560 390];
    styles = {'-','--', '-.'};
    colors = [0 0.4470  0.7410; 0.8500 0.3250 0.0980; 0.9290 0.6940 0.1250; 0.4940 0.1840 0.5560];
    ax = axes('XScale', 'log', 'YScale', 'linear');
    ax.LineStyleOrder = styles;
    ax.ColorOrder = colors;
    hold on;

    for stage = 0:2

        files = dir(strcat("data_", program_name, "_*_TSL_", sprintf("%02d", num_cores),"_stage", string(stage),"_out.txt"));
        sum_latency = [];
        for file = files'
            data = importdata(file.name);
            new_length = max(length(sum_latency), length(data));

            sum_latency = [sum_latency; zeros(new_length - length(sum_latency), 1)] + [data; zeros(new_length - length(data), 1)];
        end


        plot(get_cumulative_percentages(sum_latency), "LineWidth", 1.5);

    end


    legend({"Stage 1", "Stage 2", "Stage 3"}, "Location", "southoutside", "NumColumns", 3);
    xlabel("Latency (µs)");
    xlim([0 1000000000])
    ylabel("Latency distribution");
    ytickformat('percentage');

    if num_cores == 1
        title(strcat("Task switching latency - ", display_name, " - ", string(num_cores), " core"));
    else
        title(strcat("Task switching latency - ", display_name, " - ", string(num_cores), " cores"));
    end

    fig.Position = [1000 818 560 390];

    exportgraphics(fig, strcat("res_tsl_", program_name, "_", string(num_cores), ".pdf"),"ContentType","vector");
    close all;
end


function [Cores, Overhead, Throughput] = parse_avgs(program_name)

    Throughput_tmp = [];
    Done_work = [];
    Ideal_work = [];
    Cores = [];

    files = dir(strcat("data_", program_name, "_*_AVGS_out.txt"));
    for file = files'
        data = importdata(file.name);

        if isequal(Cores, [])
            Cores = data(:,1);
        end
        assert(isequal(Cores, data(:,1)));

        Throughput_tmp = [Throughput_tmp data(:,2)];
        Done_work = [Done_work data(:,3)];
        Ideal_work = [Ideal_work data(:,4)];
    end

    Overhead = (1 - sum(Done_work, 2) ./ sum(Ideal_work, 2)) * 100;
    Throughput = harmmean(Throughput_tmp, 2);
end


function plot_avgs()
    disp("Starting AVGS");

    Legend = ["Evgen" "Evgen\_rust" "Eventdev\_pipeline" "Eventdev\_pipeline\_rust"];
    Overhead = [];
    Throughput = [];

    [Cores, Overhead_tmp, Throughput_tmp] = parse_avgs("evgen");
    Overhead = [Overhead Overhead_tmp];
    Throughput = [Throughput Throughput_tmp];

    [Cores_tmp, Overhead_tmp, Throughput_tmp] = parse_avgs("revgen");
    Overhead = [Overhead Overhead_tmp];
    Throughput = [Throughput Throughput_tmp];
    assert(isequal(Cores, Cores_tmp));

    [Cores_tmp, Overhead_tmp, Throughput_tmp] = parse_avgs("eventdev");
    Overhead = [Overhead Overhead_tmp];
    Throughput = [Throughput Throughput_tmp];
    assert(isequal(Cores, Cores_tmp));

    [Cores_tmp, Overhead_tmp, Throughput_tmp] = parse_avgs("reventdev");
    Overhead = [Overhead Overhead_tmp];
    Throughput = [Throughput Throughput_tmp];
    assert(isequal(Cores, Cores_tmp));

    fig = figure();
    colors = [0 0.4470  0.7410; 0.8500 0.3250 0.0980; 0.9290 0.6940 0.1250; 0.4940 0.1840 0.5560];
    ax.ColorOrder = colors;
    plot(Throughput, "LineWidth",1.5);
    legend(Legend, "Location", "southoutside", "NumColumns", 2);
    xlabel("Number of worker cores");
    ylabel("Throughput (Mp/s)");
    title("Throughput")
    fig.Position = [1000 818 560 390];

    exportgraphics(fig, strcat("res_average_throughput.pdf"),"ContentType","vector");

    fig = figure();
    colors = [0 0.4470  0.7410; 0.8500 0.3250 0.0980; 0.9290 0.6940 0.1250; 0.4940 0.1840 0.5560];
    ax.ColorOrder = colors;
    plot(Overhead, "LineWidth",1.5);
    legend(Legend, "Location", "southoutside", "NumColumns", 2);
    xlabel("Number of worker cores");
    ylabel("Overhead");
    ylim([0 100]);
    ytickformat('percentage');
    title("Processing overhead");
    fig.Position = [1000 818 560 390];

    exportgraphics(fig, strcat("res_average_overhead.pdf"),"ContentType","vector");

    fig = figure();
    colors = [0 0.4470  0.7410; 0.8500 0.3250 0.0980; 0.9290 0.6940 0.1250; 0.4940 0.1840 0.5560];
    ax.ColorOrder = colors;
    plot(Throughput ./ Throughput(1,:) ./ [1:28]' , "LineWidth",1.5);
    legend(Legend, "Location", "southoutside", "NumColumns", 2);
    xlabel("Number of worker cores");
    ylabel("Parallel efficiency");
    title("Parallel efficiency")
    fig.Position = [1000 818 560 390];

    exportgraphics(fig, strcat("res_parallel_efficiency.pdf"),"ContentType","vector");
end
