select count(*) from imdb100, imdb121, imdb88 where imdb100.d = imdb121.d and imdb121.d = imdb88.s;