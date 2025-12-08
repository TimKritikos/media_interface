## media interface

An abstraction layer for the file layout different cameras or other devices use to store their data. This is a companion to the media organizer project which allows that to be camera/device agnostic.

It is possible to query for items and get a response like this `interface -l /mnt/MEDIA/source_media/GoPro_Hero_13_Black_SN:12345678/DATA/CARD0009/`

```json
{
  "data_type": "source_media_interface_api",
  "version": "0.1.0",
  "command_success": true,
  "file_list": [
    {
      "file_path": "/mnt/MEDIA/source_media/GoPro_Hero_13_Black_SN:12345678/DATA/CARD0009/GX010212.THM",
      "file_type": "image-preview",
      "item_type": "video",
      "metadata_file": "/mnt/MEDIA/source_media/GoPro_Hero_13_Black_SN:12345678/DATA/CARD0009/GX010212.MP4"
    },
    {
      "file_path": "/mnt/MEDIA/source_media/GoPro_Hero_13_Black_SN:12345678/DATA/CARD0009/GOPR0210.JPG",
      "file_type": "image",
      "item_type": "image"
    }
  ]
}
```
Each item is represented by a file, in case of -l with the lowest quality representation and -L with the highest, the type of the file, the type of the item, and in case the file doesn't have the embedded metadata of the item, a file that does.

It is possible to then query for all the files representing an item by providing any one of them, for example `interface -g /mnt/MEDIA/source_media/GoPro_Hero_13_Black_SN:12345678/DATA/CARD0009/GX010212.THM`
```json
{
  "data_type": "source_media_interface_api",
  "version": "0.1.0",
  "command_success": true,
  "file_list": [
    {
      "file_path": "/mnt/MEDIA/source_media/GoPro_Hero_13_Black_SN:12345678/DATA/CARD0009/GX010212.MP4",
      "file_type": "video",
      "item_type": "video",
      "part_count": 2,
      "part_num": 1
    },
    {
      "file_path": "/mnt/MEDIA/source_media/GoPro_Hero_13_Black_SN:12345678/DATA/CARD0009/GL010212.LRV",
      "file_type": "video-preview",
      "item_type": "video",
      "part_count": 2,
      "part_num": 1
    },
    {
      "file_path": "/mnt/MEDIA/source_media/GoPro_Hero_13_Black_SN:12345678/DATA/CARD0009/GX010212.THM",
      "file_type": "image-preview",
      "item_type": "video",
      "part_count": 2,
      "part_num": 1
    },
    {
      "file_path": "/mnt/MEDIA/source_media/GoPro_Hero_13_Black_SN:12345678/DATA/CARD0009/GX020212.MP4",
      "file_type": "video",
      "item_type": "video",
      "part_count": 2,
      "part_num": 2
    },
    {
      "file_path": "/mnt/MEDIA/source_media/GoPro_Hero_13_Black_SN:12345678/DATA/CARD0009/GL020212.LRV",
      "file_type": "video-preview",
      "item_type": "video",
      "part_count": 2,
      "part_num": 2
    },
    {
      "file_path": "/mnt/MEDIA/source_media/GoPro_Hero_13_Black_SN:12345678/DATA/CARD0009/GX020212.THM",
      "file_type": "image-preview",
      "item_type": "video",
      "part_count": 2,
      "part_num": 2
    }
  ]
}
```
Here it includes both parts of the video and the thumbnail and low bitrate proxie clip GoPro creates for each.

To configure this program you need to specify which media handler is on what directory, for example
```json
{
	"data_type": "source_media_config",
	"source_media": [
		{
			"path": "/mnt/MEDIA/source_media/GoPro_Hero_13_Black_SN:12345678/",
			"card_subdir":"DATA",
			"handler": "GoPro-Hero-Generic-1"
		},{
			"path": "/mnt/MEDIA/source_media/Sony_ILCEM4_SN:12345678/",
			"card_subdir":"DATA",
			"handler": "Sony-ILCEM4-1"
		}
	]
}
```

It is also possible to specify known missing files in per-source-media config files, for example
```json
{
	"data_type": "source_media_config",
    "errata": {
		"known_missing_files": [
			"/mnt/MEDIA/source_media/GoPro_Hero_13_Black_SN:12345678/DATA/CARD0061/GL011273.LRV",
		]
	}

}
```
