select count(*) from imdb100, imdb119, imdb88 where imdb100.d = imdb119.d and imdb119.d = imdb88.s;