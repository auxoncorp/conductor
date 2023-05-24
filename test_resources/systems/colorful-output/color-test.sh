#!/bin/bash

# 256 Color
# From https://askubuntu.com/a/1260375

for i in {0..255} ; do
    # Black FG on color BG
    printf "\e[30;48;5;%sm%4d " "$i" "$i"

    # White FG on color BG
    printf "\e[97m%4d " "$i"

    # Color FG on black BG
    printf "\e[40;38;5;%sm%4d " "$i" "$i"

    # Color FG on white BG
    printf "\e[107m%4d " "$i"

    # Check whether to print new line
    [ $(( ($i +  1) % 4 )) == 0 ] && set1=1 || set1=0
    [ $(( ($i - 15) % 6 )) == 0 ] && set2=1 || set2=0
    if ( (( set1 == 1 )) && (( i <= 15 )) ) || ( (( set2 == 1 )) && (( i > 15 )) ); then
        printf "\e[0m\n";
    fi
done


# RGB Color
# From https://unix.stackexchange.com/a/404415

awk -v term_cols="${width:-$(tput cols || echo 80)}" 'BEGIN{
    s="/\\";
    for (colnum = 0; colnum<term_cols; colnum++) {
        r = 255-(colnum*255/term_cols);
        g = (colnum*510/term_cols);
        b = (colnum*255/term_cols);
        if (g>255) g = 510-g;
        printf "\033[48;2;%d;%d;%dm", r,g,b;
        printf "\033[38;2;%d;%d;%dm", 255-r,255-g,255-b;
        printf "%s\033[0m", substr(s,colnum%2+1,1);
    }
    printf "\n";
}'
