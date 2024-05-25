# Summary

I had an external SSD that I wanted to make a couple of FAT32 filesystems on,
so that they could be read on Windows, but I wanted to make sure they would
show up as read-only on Windows. (*Actually, I was hoping to make it read-only
for MacOS too, but I did not succeed.*)

According to wikipedia, the way you do this (for Windows) is by setting bit 60
in the partition flags. Source:
[wikipedia](https://web.archive.org/web/20240507233156/https://en.wikipedia.org/wiki/Microsoft_basic_data_partition)

So I needed a way to set bit 60 in the partition flags. There doesn't seem
to be a good rust crate that understands GPT and that also works with actual
disk devices in `/dev`. There is [one crate](https://crates.io/crates/gpt) that
seems to mostly be targeted to reading and writing disk images, within a normal
filesystem that doesn't require sector-aligned I/O. There is
[another crate](https://crates.io/crates/gptman) that just punts on the whole
question of I/O and makes you do it all yourself. Between those two, I chose
to use the latter, and I had to implement the sector-aligned I/O myself.

## Note regarding MacOS

Windows respects bit 60, but apparently MacOS doesn't. I was not able to
find a suitable way to make the filesystem read-only on MacOS. In particular,
even if you don't intentionally change any files, MacOS keeps making
directories called `.Spotlight-V100` and `.fseventsd`. I tried making both
of those as files, so that MacOS couldn't make them as directories. That
trick worked for `.Spotlight-V100`, but MacOS just deleted my `.fseventsd`
file and re-made it as a directory.

I think I just have to make a rule that the SSD can never be connected to
a Mac.

# Usage (assuming this is being done on FreeBSD)

I connected the SSD. It shows up as da0, according to dmesg:

```
> dmesg | tail -17
usb_msc_auto_quirk: UQ_MSC_NO_PREVENT_ALLOW set for USB mass storage device SanDisk Extreme Pro 55AF (0x0781:0x55af)
ugen0.4: <SanDisk Extreme Pro 55AF> at usbus0
umass0 on uhub2
umass0: <SanDisk Extreme Pro 55AF, class 0/0, rev 2.10/40.60, addr 6> on usbus0
umass0:  SCSI over Bulk-Only; quirks = 0x8000
umass0:3:0: Attached to scbus3
da0 at umass-sim0 bus 0 scbus3 target 0 lun 0
da0: <SanDisk Extreme Pro 55AF 4060> Fixed Direct Access SPC-4 SCSI device
da0: Serial Number 323431313931343031363633
da0: 40.000MB/s transfers
da0: 3815415MB (7813971617 512 byte sectors)
da0: quirks=0x2<NO_6_BYTE>
ses0 at umass-sim0 bus 0 scbus3 target 0 lun 1
ses0: <SanDisk SES Device 4060> Fixed Enclosure Services SPC-4 SCSI device
ses0: Serial Number 323431313931343031363633
ses0: 40.000MB/s transfers
ses0: SES Device
> 
```

Delete the partition table, then make a fresh GPT partition table:

```
# gpart destroy -F da0
da0 destroyed
# gpart create -s GPT da0
da0 created
# gpart show da0
=>        40  7813971544  da0  GPT  (3.6T)
          40  7813971544       - free -  (3.6T)

# 
```

My belief is that FAT32 filesystems have a maximum size of 2TB. `newfs_msdos`
seems to be happy to create bigger ones, but that could be a bug in
`newfs_msdos`. In fact, based on my reading of the struct definitions, I
don't even think you can quite get all the way to 2TB, I think the max
is probably (2^32 - 1) sectors.

```
# gpart add -s 3519004249 -t ms-basic-data -l 'Media collection: Audio' da0
da0p1 added
# gpart add -s 4294967295 -t ms-basic-data -l 'Media collection: Video' da0
da0p2 added
# 
```

Make filesystems:

```
# newfs_msdos -L wuff_media1 /dev/da0p1
/dev/da0p1: 3518145280 sectors in 54971020 FAT32 clusters (32768 bytes/cluster)
BytesPerSec=512 SecPerClust=64 ResSectors=32 FATs=2 Media=0xf0 SecPerTrack=63 Heads=255 HiddenSecs=0 HugeSectors=3519004249 FATsecs=429462 RootCluster=2 FSInfo=1 Backup=2
# newfs_msdos -L wuff_media2 /dev/da0p2
/dev/da0p2: 4293918912 sectors in 67092483 FAT32 clusters (32768 bytes/cluster)
BytesPerSec=512 SecPerClust=64 ResSectors=32 FATs=2 Media=0xf0 SecPerTrack=63 Heads=255 HiddenSecs=0 HugeSectors=4294967295 FATsecs=524161 RootCluster=2 FSInfo=1 Backup=2
# 
```

Set them to read-only:

```
# ./target/debug/toggle-ro /dev/da0
Partition 1: Media collection: Audio
Read only? [n] y

Partition 2: Media collection: Video
Read only? [n] y

# 
```

# Appendix: adding to fstab (again assuming FreeBSD)

The `-L` arguments that we gave to `newfs_msdos` are what is used to
populate the `/dev/msdosfs` directory. Except they are turned, inexplicably,
into all capital letters.

So (assuming there is a group named "media"), we can put into `/etc/fstab`
something like the following:

```
/dev/msdosfs/WUFF_MEDIA1 /u/media/audio msdosfs ro,noauto,-u=0,-g=media,-m=664,-M=775 0 0
/dev/msdosfs/WUFF_MEDIA2 /u/media/video msdosfs ro,noauto,-u=0,-g=media,-m=664,-M=775 0 0
```

Then `mount /u/media/audio && mount /u/media/video` will mount them both
read-only, and the mounts can be upgraded to read-write when desired, with:
`mount -uw /u/media/audio && mount -uw /u/media/video`

If fscking is desired, the `fsck_msdos` utility conveniently accepts multiple
filesystems on the command line: `fsck_msdosfs /dev/msdosfs/WUFF_MEDIA*`
