import enum
import json

from flask import Flask, request
from flask_restful import Resource, Api, reqparse
from flask_sqlalchemy import SQLAlchemy
from flask_dotenv import DotEnv
from flask_marshmallow import Marshmallow
from flask_migrate import Migrate
from flask_uuid import FlaskUUID
from marshmallow_enum import EnumField
from marshmallow_sqlalchemy import auto_field
import uuid

from sqlalchemy.dialects.postgresql import UUID

import boto3
import werkzeug.datastructures

app = Flask(__name__)
api = Api(app)
env = DotEnv(app)
db = SQLAlchemy(app)
ma = Marshmallow(app)
flask_uuid = FlaskUUID(app)
migrate = Migrate(app, db)

app.config["MAX_CONTENT_LENGTH"] = int(app.config["BACKEND_MAX_CONTENT_LENGTH"])

MAX_STRING_LENGTH = app.config["BACKEND_MAX_STRING_LENGTH"]

reqparser = reqparse.RequestParser()
reqparser.add_argument("audio", werkzeug.datastructures.FileStorage, location="files")

session = boto3.session.Session(
    aws_access_key_id=app.config["S3_ACCESS_KEY_ID"],
    aws_secret_access_key=app.config["S3_SECRET_ACCESS_KEY"],
)
s3 = session.resource("s3",
                      endpoint_url=app.config["S3_ENDPOINT_URL"])

bucket = s3.Bucket(app.config["S3_BUCKET_NAME"])

extra_s3_args = {
    "ACL": "public-read",
    "CacheControl": app.config["BACKEND_UPLOAD_CACHE_CONTROL"],
    "ContentType": "audio/ogg",
}

class Privacy(enum.Enum):
    Public = 1
    Unlisted = 2

class Recording(db.Model):
    id = db.Column(UUID(as_uuid=True), primary_key=True)
    name = db.Column(db.String(MAX_STRING_LENGTH), nullable=False)
    category_id = db.Column(db.SmallInteger, db.ForeignKey("category.id"), nullable=False)

    privacy = db.Column(db.Enum(Privacy), nullable=False)
    age_id = db.Column(db.SmallInteger, db.ForeignKey("age.id"), nullable=True)
    gender_id = db.Column(db.SmallInteger, db.ForeignKey("gender.id"), nullable=True)
    location = db.Column(db.String(MAX_STRING_LENGTH), nullable=True)
    occupation = db.Column(db.String(MAX_STRING_LENGTH), nullable=True)

    parent_id = db.Column(UUID(as_uuid=True), db.ForeignKey("recording.id"), nullable=True)

class Category(db.Model):
    id = db.Column(db.SmallInteger, primary_key=True)
    label = db.Column(db.String(MAX_STRING_LENGTH), nullable=False)

class Age(db.Model):
    id = db.Column(db.SmallInteger, primary_key=True)
    label = db.Column(db.String(MAX_STRING_LENGTH), nullable=False)

class Gender(db.Model):
    id = db.Column(db.SmallInteger, primary_key=True)
    label = db.Column(db.String(MAX_STRING_LENGTH), nullable=False)

class RecordingSchema(ma.SQLAlchemySchema):
    class Meta:
        model = Recording
        fields = ("id", "name", "category_id", "privacy", "age_id", "gender_id", "location", "occupation", "parent_id")

    id = auto_field(dump_only=True)
    privacy = EnumField(Privacy)

recording_schema = RecordingSchema()
recordings_schema = RecordingSchema(many=True)

class NewRecordingResource(Resource):
    def post(self):
        from mutagen.oggopus import OggOpus

        # the data is a multipart stream containing the metadata and
        # the Opus data in an Ogg container

        stream = request.files["audio"].stream
        # try to open the file to make sure it's an Ogg file
        opus_file = OggOpus(stream)
        stream.seek(0)

        metadata = json.loads(request.form["metadata"])
        recording = recording_schema.load(metadata)

        new_id = str(uuid.uuid4())

        db.session.add(Recording(id=new_id, **recording))
        db.session.commit()

        bucket.upload_fileobj(stream, f"{new_id}.ogg", ExtraArgs=extra_s3_args)

        return recording_schema.dump(recording), 201

api.add_resource(NewRecordingResource, "/recordings/")

class RecordingResource(Resource):
    def get(self, id):
        recording = db.session.query(Recording).get(id)

        if recording:
            return recording_schema.dump(recording), 200

        return None, 404

api.add_resource(RecordingResource, "/recordings/<uuid:id>/")

class RecordingChildrenResource(Resource):
    def get(self, parent_id):
        recordings = db.session.query(Recording).filter(Recording.parent_id == parent_id).all()

        if len(recordings) > 0:
            return recordings_schema.dump(recordings), 200

        return None, 404

api.add_resource(RecordingChildrenResource, "/recordings/<uuid:parent_id>/children/")

if __name__ == "__main__":
    app.run(debug=True)
