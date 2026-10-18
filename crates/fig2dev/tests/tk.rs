#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `tk` and `ptk` drivers against output of the real fig2dev
//! 3.2.9a (the expected texts were written by `/opt/homebrew/bin/fig2dev
//! -L tk` and `-L ptk`).

mod pic_common;
mod tex_common;

use tex_common::{compare_with_fig2dev, run, HAND};

const FIG: &str = r#"0 32 #336699
2 1 1 2 0 7 50 -1 -1 4.000 0 0 -1 1 1 4
	1 1 1.00 60.00 120.00
	2 1 1.00 60.00 120.00
	 600 600 1800 900 2400 2400 3000 2450
2 2 0 1 4 32 50 -1 43 0.000 0 0 -1 0 0 5
	 3600 600 4800 600 4800 1500 3600 1500 3600 600
1 3 0 1 0 7 50 -1 25 0.000 1 0.0000 2400 3600 600 600 2400 3600 3000 3600
5 1 0 1 1 7 50 -1 -1 0.000 0 1 0 0 3600.0 3600.0 3000 3600 3600 3000 4200 3600
4 1 4 50 -1 14 12 0.0000 4 135 600 600 4800 say "hi" it's\001
"#;

#[test]
fn tk_default() {
    let (out, _msgs, exit) = run("tk", FIG, &[]).unwrap();
    assert_eq!(exit, 0);
    assert_eq!(
        out,
        r###"# Produced by fig2dev Version 3.2.9a
# The canvas name (".c") can be changed to anything you like.  It only
# occurs in the following line.  The canvas size can be changed as well.

set xfigCanvas [canvas .c -width 3.47i -height 3.27i]
$xfigCanvas config -xscrollincrement 1p -yscrollincrement 1p
# Shift canvas by lower of bounding box
$xfigCanvas xview scroll 14 u
$xfigCanvas yview scroll 33 u
pack $xfigCanvas

proc tint {color percent} {
    foreach {red green blue} [winfo rgb . $color] {
	set red [expr {255 - ((65535-$red)/256)*$percent/100}]
	set green [expr {255 - ((65535-$green)/256)*$percent/100}]
	set blue [expr {255 - ((65535-$blue)/256)*$percent/100}]
	# break
    }
    return [format "#%02x%02x%02x" $red $green $blue]
}

# If the files defining the stipple bitmaps should not be placed
# into the current directory, change the next line appropriately.
set xbmPath .

# crosshatch30
set xfigp2 [file join $xbmPath crosshatch30.xbm]
if { [file exist $xfigp2] == 0 } {
    set fd [open $xfigp2 w]
    puts $fd {#define crosshatch30_width 8
#define crosshatch30_height 4
static unsigned char crosshatch30_bits[] = {
	0x81, 0x66, 0x18, 0x66};}
close $fd
}
# The xfig objects begin here.
$xfigCanvas create arc 2.250i 2.250i 3.150i 3.150i -start 180.000000 -extent 180.000000 \
  -style arc -fill {} -outline #0000ff
$xfigCanvas create oval 1.350000i 2.250000i 2.250000i \
  3.150000i -fill [tint #ffffff 75] -outline #000000
$xfigCanvas create line 0.450000i \
  0.450000i 1.350000i 0.675000i 1.800000i 1.800000i 2.250000i 1.837500i -arrow both \
  -arrowshape {0.072000i 0.090000i 0.022500i} -dash {5 5} -width 2
$xfigCanvas create \
  polygon 2.700000i 0.450000i 3.600000i 0.450000i 3.600000i 1.125000i 2.700000i 1.125000i \
  -fill #336699 -outline {} -width 0
$xfigCanvas create polygon 2.700000i 0.450000i 3.600000i 0.450000i 3.600000i 1.125000i \
  2.700000i 1.125000i -fill #ff0000 -stipple @$xfigp2 -outline #ff0000
$xfigCanvas create text 0.450000i 3.600000i -text "say \"hi\" it's" \
 -anchor s -font "-adobe-courier-bold-r-normal--12-0-0-0-m-0-iso8859-1" -fill #ff0000
focus $xfigCanvas
"###
    );
}

#[test]
fn ptk_default() {
    let (out, _msgs, exit) = run("ptk", FIG, &[]).unwrap();
    assert_eq!(exit, 0);
    assert_eq!(
        out,
        r###"sub {
my $top = shift;
my %img;
my $c = $top->Canvas(qw/-width 3.85i -height 3.64i -bg ivory/);
#$c->configure(qw/-xscrollincrement 1p -yscrollincrement 1p/);
$c->configure(-scrollregion => ['0.19i','0.45i','4.05i','4.08i']);
# Shift canvas by lower of bounding box
#$c->xview(qw/scroll 14 u/);
#$c->yview(qw/scroll 33 u/);
$c->pack(-expand => 1, -fill => 'both');

# Shade and tint colors: &$shade($color, shade), &$tint($color, tint),
# where shade and tint are between 0 and 100, respectively.
my $shade = sub {
    my($r, $g, $b, $c, $s);
    ($c, $s) = @_;    $s /= 100;
    ($r, $g, $b) = $top->rgb($c);
    return(sprintf('#%.2x%.2x%.2x', $r*$s/256, $g*$s/256, $b*$s/256));
};
my $tint = sub {
    my($r, $g, $b, $c, $t);
    ($c, $t) = @_;    $t /= 100;
    ($r, $g, $b) = $top->rgb($c);
    return(sprintf('#%.2x%.2x%.2x', 255 - (65535-$r)*$t/256,
                   255 - (65535-$g)*$t/256, 255 - (65535-$b)*$t/256));
};

# If the fill-pattern bitmaps should not be placed into
# the current directory, change the next line accordingly.
my $xbmPath = '.';

# crosshatch30
my $xfigp2 = "$xbmPath/crosshatch30.xbm";
unless (-e $xfigp2) {
    open(my $fh, '>', $xfigp2) or die "Cannot open file '$xfigp2' for writing: $!\n";
    print $fh <<'EOF';
#define crosshatch30_width 8
#define crosshatch30_height 4
static unsigned char crosshatch30_bits[] = {
	0x81, 0x66, 0x18, 0x66};
EOF
    close $fh;
};

# The xfig objects begin here.
$c->createArc(qw/2.500i 2.500i 3.500i 3.500i -start 180.000000 -extent 180.000000/,
    -style => 'arc', -fill => '', -outline => '#0000ff');
$c->createOval(qw/1.500000i 2.500000i 2.500000i 3.500000i/, -fill => &$tint('#ffffff',
    75), -outline => '#000000');
$c->createLine('0.500000i','0.500000i', '1.500000i','0.750000i', '2.000000i','2.000000i',
    '2.500000i','2.041667i', -arrow => 'both', -arrowshape => ['0.080000i', '0.100000i',
    '0.025000i'], -dash => [5,5], -width => 2);
$c->createPolygon('3.000000i','0.500000i', '4.000000i','0.500000i', '4.000000i','1.250000i',
    '3.000000i','1.250000i', -fill => '#336699', -outline => undef, -width => 0);
$c->createPolygon('3.000000i','0.500000i', '4.000000i','0.500000i', '4.000000i','1.250000i',
    '3.000000i','1.250000i', -fill => '#ff0000', -stipple => "\@$xfigp2", -outline
    => '#ff0000');
$c->createText(qw/0.500000i 4.000000i -text/, 'say "hi" it\'s',, -anchor => 's', -font => "-adobe-courier-bold-r-normal--12-0-0-0-m-0-iso8859-1", -fill => '#ff0000');

}
"###
    );
}

#[test]
fn tk_full_page_landscape_background() {
    let (out, _msgs, exit) = run(
        "tk",
        FIG,
        &[('g', Some("#ff00ff")), ('P', None), ('z', Some("A4")), ('l', Some("x"))],
    )
    .unwrap();
    assert_eq!(exit, 0);
    assert_eq!(
        out,
        r###"# Produced by fig2dev Version 3.2.9a
# The canvas name (".c") can be changed to anything you like.  It only
# occurs in the following line.  The canvas size can be changed as well.

# Page size specified: A4
# Landscape orientation
set xfigCanvas [canvas .c -width 10.52i -height 7.44i] -bg #10000100
$xfigCanvas config -xscrollincrement 1p -yscrollincrement 1p
pack $xfigCanvas

proc tint {color percent} {
    foreach {red green blue} [winfo rgb . $color] {
	set red [expr {255 - ((65535-$red)/256)*$percent/100}]
	set green [expr {255 - ((65535-$green)/256)*$percent/100}]
	set blue [expr {255 - ((65535-$blue)/256)*$percent/100}]
	# break
    }
    return [format "#%02x%02x%02x" $red $green $blue]
}

# If the files defining the stipple bitmaps should not be placed
# into the current directory, change the next line appropriately.
set xbmPath .

# crosshatch30
set xfigp2 [file join $xbmPath crosshatch30.xbm]
if { [file exist $xfigp2] == 0 } {
    set fd [open $xfigp2 w]
    puts $fd {#define crosshatch30_width 8
#define crosshatch30_height 4
static unsigned char crosshatch30_bits[] = {
	0x81, 0x66, 0x18, 0x66};}
close $fd
}
# The xfig objects begin here.
$xfigCanvas create arc 2.250i 2.250i 3.150i 3.150i -start 180.000000 -extent 180.000000 \
  -style arc -fill {} -outline #0000ff
$xfigCanvas create oval 1.350000i 2.250000i 2.250000i \
  3.150000i -fill [tint #ffffff 75] -outline #000000
$xfigCanvas create line 0.450000i \
  0.450000i 1.350000i 0.675000i 1.800000i 1.800000i 2.250000i 1.837500i -arrow both \
  -arrowshape {0.072000i 0.090000i 0.022500i} -dash {5 5} -width 2
$xfigCanvas create \
  polygon 2.700000i 0.450000i 3.600000i 0.450000i 3.600000i 1.125000i 2.700000i 1.125000i \
  -fill #336699 -outline {} -width 0
$xfigCanvas create polygon 2.700000i 0.450000i 3.600000i 0.450000i 3.600000i 1.125000i \
  2.700000i 1.125000i -fill #ff0000 -stipple @$xfigp2 -outline #ff0000
$xfigCanvas create text 0.450000i 3.600000i -text "say \"hi\" it's" \
 -anchor s -font "-adobe-courier-bold-r-normal--12-0-0-0-m-0-iso8859-1" -fill #ff0000
focus $xfigCanvas
"###
    );
}

#[test]
fn ptk_full_page_wrapped() {
    let (out, _msgs, exit) = run("ptk", FIG, &[('P', None), ('z', Some("A4")), ('w', None)]).unwrap();
    assert_eq!(exit, 0);
    assert_eq!(
        out,
        r###"# Page size specified: A4
# Landscape orientation
use strict;
use warnings;
use Tk;
my $xfigure = sub {
my $top = shift;
my %img;
my $c = $top->Canvas(qw/-width 10.52i -height 7.44i -bg ivory/);
#$c->configure(qw/-xscrollincrement 1p -yscrollincrement 1p/);
$c->pack(-expand => 1, -fill => 'both');

# Shade and tint colors: &$shade($color, shade), &$tint($color, tint),
# where shade and tint are between 0 and 100, respectively.
my $shade = sub {
    my($r, $g, $b, $c, $s);
    ($c, $s) = @_;    $s /= 100;
    ($r, $g, $b) = $top->rgb($c);
    return(sprintf('#%.2x%.2x%.2x', $r*$s/256, $g*$s/256, $b*$s/256));
};
my $tint = sub {
    my($r, $g, $b, $c, $t);
    ($c, $t) = @_;    $t /= 100;
    ($r, $g, $b) = $top->rgb($c);
    return(sprintf('#%.2x%.2x%.2x', 255 - (65535-$r)*$t/256,
                   255 - (65535-$g)*$t/256, 255 - (65535-$b)*$t/256));
};

# If the fill-pattern bitmaps should not be placed into
# the current directory, change the next line accordingly.
my $xbmPath = '.';

# crosshatch30
my $xfigp2 = "$xbmPath/crosshatch30.xbm";
unless (-e $xfigp2) {
    open(my $fh, '>', $xfigp2) or die "Cannot open file '$xfigp2' for writing: $!\n";
    print $fh <<'EOF';
#define crosshatch30_width 8
#define crosshatch30_height 4
static unsigned char crosshatch30_bits[] = {
	0x81, 0x66, 0x18, 0x66};
EOF
    close $fh;
};

# The xfig objects begin here.
$c->createArc(qw/2.500i 2.500i 3.500i 3.500i -start 180.000000 -extent 180.000000/,
    -style => 'arc', -fill => '', -outline => '#0000ff');
$c->createOval(qw/1.500000i 2.500000i 2.500000i 3.500000i/, -fill => &$tint('#ffffff',
    75), -outline => '#000000');
$c->createLine('0.500000i','0.500000i', '1.500000i','0.750000i', '2.000000i','2.000000i',
    '2.500000i','2.041667i', -arrow => 'both', -arrowshape => ['0.080000i', '0.100000i',
    '0.025000i'], -dash => [5,5], -width => 2);
$c->createPolygon('3.000000i','0.500000i', '4.000000i','0.500000i', '4.000000i','1.250000i',
    '3.000000i','1.250000i', -fill => '#336699', -outline => undef, -width => 0);
$c->createPolygon('3.000000i','0.500000i', '4.000000i','0.500000i', '4.000000i','1.250000i',
    '3.000000i','1.250000i', -fill => '#ff0000', -stipple => "\@$xfigp2", -outline
    => '#ff0000');
$c->createText(qw/0.500000i 4.000000i -text/, 'say "hi" it\'s',, -anchor => 's', -font => "-adobe-courier-bold-r-normal--12-0-0-0-m-0-iso8859-1", -fill => '#ff0000');

};
my $mw = MainWindow->new;
$xfigure->($mw);
MainLoop;
"###
    );
}

#[test]
fn unknown_options_and_paper_sizes_fail() {
    assert!(run("tk", FIG, &[('w', None)]).is_err());
    assert!(run("ptk", FIG, &[('g', Some("red"))]).is_err());
    assert!(run("tk", FIG, &[('P', None), ('z', Some("bogus"))]).is_err());
}

#[test]
fn hostile_values_dont_panic() {
    let bodies = [
        "2 1 1 1 0 7 50 -1 -1 0.001 0 0 -1 1 1 2
	1 1 1.00 60.00 120.00
	1 1 1.00 60.00 120.00
	 0 0 2147483647 -2147483648
2 2 3 1 0 7 50 -1 -1 0.010 0 0 -1 0 0 5
	 -2147483648 0 2147483647 0 2147483647 9 0 9 0 0
2 4 1 1 0 7 50 -1 -1 4.0 0 0 0 0 0 5
	 0 0 0 0 0 0 0 0 0 0
1 1 1 1 0 7 50 -1 -1 4.0 1 1e300 0 0 5 5 0 0 0 0
",
        "5 1 2 4 7 2 0 -1 0 0 0 0 1 1 79E6 12003420 79196029 3765 5984 3270 6254 51752685
	1 1 2 120 240
	1 1 2 120 240
2 1 0 1 0 7 50 -1 -1 0.0 0 0 -1 1 1 1
	14 1 1.0 -60.0 -10.0
	14 0 1.0 60.0 0.0
	 -2147483648
2 3 0 1 0 7 50 -1 -1 0.0 0 0 -1 0 0 3
	 0 0 0 0 0 0
4 1 0 50 -1 0 12 1e300 4 135 600 -2147483648 2147483647 x\\001
",
    ];
    for body in bodies {
        for lang in ["tk", "ptk"] {
            for opts in [
                &[][..],
                &[('P', None)][..],
                &[('m', Some("0"))][..],
                &[('G', Some("1:1"))][..],
            ] {
                let _ = run(lang, body, opts);
            }
        }
    }
}

/// Opt-in: `cargo test -p rustfig-fig2dev --test tk -- --ignored --nocapture`.
#[test]
#[ignore]
fn same_as_fig2dev() {
    let extra: Vec<&str> = HAND.to_vec();
    let opts: &[&[&str]] = &[
        &[],
        &["-P", "-z", "A4", "-l", "x"],
        &["-p", "x", "-P"],
        &["-g", "#123456"],
    ];
    // -g is Tcl/Tk only, -w Perl/Tk only (the other driver refuses it).
    for (lang, own) in [("tk", &["-g", "red", "-P"][..]), ("ptk", &["-w"][..])] {
        let mut opts = opts.to_vec();
        opts.push(own);
        if lang == "ptk" {
            opts.retain(|o| !o.contains(&"-g"));
        }
        if let Some((same, total)) = compare_with_fig2dev(lang, &extra, &opts) {
            assert!(same * 20 >= total * 19, "{lang}: {same} of {total} identical");
        }
    }
}
